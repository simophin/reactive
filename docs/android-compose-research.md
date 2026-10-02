# Android: Jetpack Compose Backend Research

Status: implemented as the Android backend; see [android.md](./android.md). Captured 2026-09-28.

This note records the investigation into whether the Android backend can target Jetpack Compose instead of (or alongside) legacy Android Views, and the design we converged on.

## Question

Can a Rust reactive core drive Compose directly? The initial assumption was "no": Compose is not a plain API surface but a compiler-plugin-driven runtime, so there is no obvious insertion point from a foreign language.

## Findings

### What is true about the assumption

- `@Composable` functions are rewritten by the Kotlin compiler plugin (hidden `Composer` parameter, `$changed` bitmasks, group calls). Calling them over JNI would depend on an unstable, undocumented ABI. Not viable.
- The Compose UI node tree (`LayoutNode`) is internal. There is no public equivalent of `addView` for creating UI nodes outside a composition.
- Composable call sites must therefore be Kotlin compiled with the Compose plugin. `dexer` cannot generate them.

### Where the real insertion point is: state

Rust does not call Compose. Rust writes to **snapshot state** (`MutableState`, `SnapshotStateList`), and a fixed set of Kotlin composables read that state. Compose then recomposes only the readers of what changed, so fine-grained reactivity is preserved end to end.

Compose's own design expects hoisted state to be owned outside composables (`TextFieldState`, `LazyListState`, `ScrollState`, ...), so Rust owning those objects is "direct integration" in Compose's own terms.

Public low-level hooks that are plain classes/interfaces (and so could be implemented from Rust via `dexer`, no Kotlin needed): `Modifier.Node` (layout/draw/pointer/semantics), `MeasurePolicy`, `MutableState`, `TextFieldState` and its transformations.

### Prior art: Redwood

[Redwood](https://github.com/cashapp/redwood) by Cash App (Block), Apache 2.0, pre-1.0.

- Uses the **Compose runtime** (not Compose UI) as a tree-diffing engine. Guest code writes `@Composable` functions against a schema-defined widget set; the resulting tree changes are serialized as a protocol and applied by a host to real native widgets (Android Views, Compose UI, UIKit, DOM).
- Its purpose is shipping UI without an app-store release: combined with **Zipline** (Kotlin/JS in embedded QuickJS, updatable at runtime) and **Treehouse** (glue/lifecycle), the downloadable guest owns logic and structure while the host owns rendering.
- Schema (`@Widget`, `@Property`, `@Children`, layout modifiers) drives codegen for guest composables, host widget interfaces and protocol serializers. Hosts implement every widget themselves; Redwood ships plumbing plus `redwood-layout` (Row/Column/Box with identical semantics per platform) and `redwood-lazylayout`.

**Why we are not using it:** Redwood's guest side (Compose runtime as reactivity and diff engine) duplicates what our Rust core already does, and its serialized protocol is exactly the indirection this project avoids. One of our core strengths is direct hooks into native frameworks: synchronous calls both ways, subclassing native extension points (e.g. `ReactiveTextStorage` subclassing `NSTextStorage`; `dex_class!` overriding `onMeasure`/`onLayout` on Android), and taking part in the real layout pass. React Native has historically been unable to do this because it never owns the real native objects.

**Worth borrowing from Redwood:** schema-plus-codegen per platform, and in particular how `redwood-layout` defines layout semantics that behave identically on Compose, Views and UIKit.

## Relation to architecture principles

At first glance a Compose backend conflicts with [architecture-principles.md](./architecture-principles.md) ("direct ownership of live UI objects", "no intermediate UI tree diff"), because Compose has no view objects to own, only state, and its runtime sits between writes and pixels.

Our conclusion: the difference is practical, not philosophical. Everything still runs synchronously on the main thread, with no serialization or protocol, and Rust holds real JVM objects and calls their setters. On Android today Compose *is* the native framework, and its unit of ownership is hoisted state.

## Proposed design: a Kotlin node object per widget

Kotlin bridging code is acceptable. Each widget gets a Kotlin node class whose setters write Compose state, giving Rust the same "object with setters" shape it already targets for Views:

```kotlin
abstract class RNode(val id: Long) {
    @Composable abstract fun Content(modifier: Modifier)
}

class LabelNode(id: Long) : RNode(id) {
    var text by mutableStateOf("")
    var fontSize by mutableStateOf(14f)

    @Composable override fun Content(modifier: Modifier) =
        Text(text, modifier, fontSize = fontSize.sp)
}
```

The Rust side only changes its binding target; `Prop`, effects and component code stay the same:

```rust
declare_jni_binding! {
    class com.reactive.compose.LabelNode {
        void setText(java.lang.String);
        void setFontSize(float);
    }
}
```

**Kotlin bridge** (in `android-lib`, Compose plugin enabled):

- `RNode` subclasses: one per widget, with state fields and `Content()`.
- Container node: `children: SnapshotStateList<RNode>`, rendered with `key(child.id) { child.Content() }`.
- Root: `ReactiveRoot(scope)` for `setContent {}`, plus a composable to embed a Rust tree inside an existing Compose app.

**Rust** keeps all reactivity, tree structure (insert/remove/move on the children list) and, optionally, layout: a container node can use `Layout(content, measurePolicy)` with a `MeasurePolicy` that calls into the Rust flex engine (`flex_layout.rs`), keeping one layout engine across platforms inside the real Compose layout pass.

Since a prop binding on the Rust side is identical for either backend, the Views vs Compose choice lives entirely in the Kotlin/`dexer` layer. Both backends can coexist.

## Practical issues and how the bridge handles them

1. **State writes land on the next frame.** With Views, `setText()` then `measure()` sees the result immediately; Compose applies writes at the next recomposition, and there is no public way to force it synchronously. Affects insert-then-scroll, text measurement, focus requests.
   *Bridge:* a node method like `whenLaidOut(callbackId)` backed by `Modifier.onPlaced` or `withFrameNanos`, giving Rust an explicit "layout is current" callback.
2. **Many Compose APIs are `suspend`** (`animateScrollToItem`, `scrollTo`); JNI cannot call them.
   *Bridge:* nodes capture `rememberCoroutineScope()` and expose plain methods such as `ListNode.scrollTo(index, animated, callbackId)` that launch the call and report completion to Rust.
3. **Composable call sites need Kotlin.**
   *Bridge:* hand-write the first 5–10 nodes; once the pattern settles, generate them from the existing descriptors via `reactive-gradle-plugin`.
4. **Writing state during composition.** Lazy list cells run Rust `setup` during composition; effects run immediately and could write state already read in the same composition (a backwards write, causing extra recomposition or loops).
   *Bridge:*
   ```kotlin
   items(count, key = ::keyAt) { i ->
       val cell = remember { nativeCreateCell(i) }
       DisposableEffect(cell) { onDispose { nativeDisposeCell(cell) } }
       cell.Content(Modifier)
   }
   ```
   Setup mostly writes to nodes it just created, which nothing has read yet. Writes to existing signals only mark them dirty, and the tick is posted through the `Handler`, so it runs after composition. Needs a test to confirm.
5. **Node identity when moved.** Moving a node to a different parent makes Compose dispose and recreate it, losing internal state.
   *Bridge:* `key(id)` covers reordering within a parent; defer cross-parent moves (`movableContentOf`) until needed. Keep state hoisted into Rust-owned objects.

Issues 1 and 4 could shape the core API, not just the Android layer, so test them first.

## Interop in both directions

- **Compose island in a Views tree:** `ComposeView` is a real `View`, so it can be a native child.
- **Rust tree in a Compose app:** wrap the root with `AndroidView` (Views backend) or the `ReactiveRoot` composable (Compose backend). Important for adoption, since most new Android apps are Compose apps.

Risk to keep in mind: Views are in maintenance mode, and new Material components and some platform features land in Compose first or only.

## Suggested first prototype

Build on Compose alongside the existing Views backend, without replacing it:

- `LabelNode`
- `ButtonNode`
- `FlexNode` container using a Rust `MeasurePolicy`
- `LazyColumn` list node driven by `RecyclingList`
- `TextFieldNode` using `TextFieldState`

Things to prove: layout calling into Rust through `MeasurePolicy` performs well, and creating Rust components during composition is safe.

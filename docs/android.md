# Android Backend (Jetpack Compose)

The Android backend renders through Jetpack Compose. It follows the design in
[android-compose-research.md](./android-compose-research.md): Rust never calls composables. It
writes Compose **snapshot state** on small Kotlin objects, and fixed composables read that state,
so Compose recomposes only what Rust changed.

## Moving parts

```
Rust (ui-core/src/android)                    Kotlin (android-lib, package com.reactive)
──────────────────────────                    ──────────────────────────────────────────
Prop setters: node.set("setText", v)  ──JNI──▶ TextNode / ButtonNode / ... (mutableStateOf)
Registries: insertChild/removeChildAt ──JNI──▶ FlexNode / StackNode .children (state list)
Taffy leaf measure                    ──JNI──▶ FlexNode.intrinsic(child, kind, cross)
FlexNode.nativeMeasure                ◀─JNI── FlexNode MeasurePolicy
NativeCallback.nativeInvoke           ◀─JNI── clicks, text edits, slider changes
ReactiveHost.nativeCreate/Tick/Destroy◀─JNI── activity lifecycle + main-looper ticks
```

Kotlin enters Rust through exactly three places:

| Kotlin                         | Rust                                   | Purpose                              |
|--------------------------------|----------------------------------------|--------------------------------------|
| `ReactiveHost`                 | `android/host.rs`, `android_main!`     | create the scope, tick, destroy      |
| `NativeCallback.invoke(value, arg)` | `android/callback.rs` (`Callback`) | every event from every widget        |
| `FlexNode.nativeMeasure`       | `android/ui/flex.rs`                   | run Taffy inside the Compose layout  |

There is no runtime class generation (dexer), no serialization, and no JNI registration step:
Kotlin `external fun`s resolve to `#[unsafe(no_mangle)]` Rust functions by name.

### Nodes

Each widget is a Kotlin class with `mutableStateOf` fields and a `Content(modifier)` composable,
for example:

```kotlin
class TextNode : Node() {
    var text by mutableStateOf("")
    var fontSize by mutableFloatStateOf(14f)
    @Composable override fun Content(modifier: Modifier) = Text(text, modifier, fontSize = fontSize.sp)
}
```

On the Rust side every widget is the shared `NativeView<Node, Node>`. A prop is a one-line setter:

```rust
pub static PROP_TEXT: Prop<Label, Node, String> = Prop::new(|n, v| n.set("setText", v.as_str()));
```

`Node::set` derives the JNI signature from the Rust argument type (`JniArg`). Supported types are
`bool`, `i32`, `i64`, `f32`, `&str`, `Option<&str>` and `&Node`.

### Events

`Callback::attach(ctx, &node, "setOnClick", |env, value, arg| ...)` boxes a Rust closure, wraps
it in a Kotlin `NativeCallback`, hands it to the node's setter, and releases it when the
component is disposed. After release, late events are ignored. What `value: Long` and
`arg: Any?` carry is up to the widget:
- **Slider:** `value` is the new value.
- **Text field:** `value` packs the UTF-16 selection and `arg` is the new text (null when only the selection moved).

### Ticks and threads

`TickScheduler` implements `std::task::Wake`. Waking it, from any thread (for example a future's
waker on a worker thread), posts `ReactiveHost.scheduleTick()` to the main looper, which calls
`nativeTick`. All UI work happens on the main thread.

### Layout

`Flex` keeps a `FlexTaffyContainer` (shared with AppKit and GTK). `FlexNode` renders
`Layout(children, measurePolicy)`, and its `MeasurePolicy`:

1. Calls `nativeMeasure`, which runs Taffy. Taffy sizes leaves through `FlexNode.intrinsic`, i.e.
   Compose intrinsic measurements. Compose allows these any number of times per pass, whereas
   `measure()` may be called only once per child.
2. Measures each child exactly once with `Constraints.fixed(...)` at the size Taffy chose and
   places it. Leaves get their content box, so padding modifiers inset them. A nested flex gets
   its border box and applies its own padding.

Going through Compose intrinsics, rather than having Rust measure a nested flex directly, also
tells Compose that the parent depends on the child's size. Compose then re-measures the parent
when a descendant's content changes.

Rust re-requests layout through `FlexNode.invalidate()` when flex props, children, or any
modifier signal read during layout changes (via `FunctionTracker`).

All values crossing into Rust are in dp, like points on Apple platforms.

## Using it

The Rust app is a `cdylib` named `reactive_android` that declares its entry point:

```rust
ui_core::android_main!(|ctx| {
    ctx.child(<Android as Platform>::Window::new("App", my_root, 0.0, 0.0));
});
```

The activity hosts it:

```kotlin
val host = ReactiveHost.create()
setContent { MaterialTheme { Surface { ReactiveContent(host) } } }
// onDestroy: host.destroy()
```

`ReactiveContent` is an ordinary composable, so a Rust tree can also be embedded inside an
existing Compose screen.

## Demo

`examples/android-demo/` holds a Rust crate (`rust/`) and a Gradle app (`app/`) wired to
`android-lib` and `reactive-gradle-plugin` through included builds:

```bash
rustup target add aarch64-linux-android
cd examples/android-demo
./gradlew installDebug      # or assembleRelease (signed with the debug key) for profiling
```

The demo covers every `Platform` widget:
- a counter with an enabled/disabled button
- a stream driven from a background thread
- a text field with Rust echo and a Rust→field command
- a slider driving a progress bar
- a Stack over an Image, and a spinner
- a reactive padding toggle
- a wrapping flex whose children are added and removed with `Show`

## Status and known limitations

Verified on a Galaxy A32 (Android 13, arm64): every demo interaction, plus repeated activity
destroy/recreate.

- **Layout cost.** Compose intrinsics are not cached across passes and are relatively slow:
  roughly 0.1 ms per `Text` query and 0.7 ms per Material `Button` on that device. A full
  re-layout of the demo's root (12 children, 3 nested flexes) takes about 15–45 ms in a
  release build. Within one pass, `FlexNode` caches sizing and intrinsic queries, and
  `measure_leaf` only asks for the extent it needs. Next steps, in order of expected payoff:
  - Default flex items to `min-size: 0`, as Yoga does. This removes the min-content queries that
    CSS's `min-size: auto` causes. It is a cross-platform behaviour change.
  - Cache leaf intrinsics across passes, invalidated per child.
  - Skip Taffy's hypothetical cross-size measurement for stretched items.
- **Lists.** `List` is not part of the `Platform` trait yet. A `LazyColumn`-backed list needs a
  test that creating components during composition (the research doc's issue 4) is safe.
- **Not yet bridged:**
  - "layout is current" callbacks (research issue 1)
  - `suspend` APIs such as scrolling (issue 2)
  - window title and back handling (`register_back_handler` is a no-op)
- **Node classes are hand-written.** That is fine at this size; generate them once the set grows.
- **Configuration changes.** The demo opts out of activity recreation for rotation. Keeping the
  host in a `ViewModel` would preserve Rust state across recreation.

# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Design Intent

Before making API-shaping or runtime-architecture changes, read [docs/architecture-principles.md](docs/architecture-principles.md).

That document captures the intended direction for:
- SolidJS-style fine-grained reactivity without a virtual DOM
- shared cross-platform widget APIs
- first-class native-view escape hatches
- why the project should prefer direct native view ownership over a React Native-style shadow tree

`AGENTS.md` covers the same ground for Codex. Keep the two in sync when the architecture moves.

## Build & Test Commands

```bash
cargo build                          # Build all (no backend features)
cargo test                           # Run all tests
cargo test <test_name>               # Run a single test
cargo test --lib                     # Library tests only
cargo build -p ui-core --features gtk --examples   # GTK backend + flex_demo (macOS/Linux)
```

Format code with `cargo fmt` after changes.

Backend features are target-gated with `compile_error!` in `ui-core/src/lib.rs`: `appkit` requires
macOS, `uikit` requires iOS, `android` requires Android, `gtk` requires macOS or Linux. A default
`cargo build` compiles no backend at all, so it will not catch breakage in platform code — build the
relevant feature explicitly. On Linux, `gtk` is the only backend you can compile or run.

## Workspace Structure

7 Rust workspace crates plus Android Gradle support:

```
core/                     — Core reactive framework (primary logic)
ui-core/                  — Cross-platform widget traits plus platform backends
resources/                — Runtime resource loading infrastructure
resources-build/          — Build-time resource utilities
android-macros/           — Android JNI binding / prop descriptor codegen
dexer/                    — DEX/class generation utilities for Android support
dexer-macros/             — Macros for DEX/class generation
android-lib/              — Kotlin Android library wrapper for ReactiveScope (not a cargo member)
reactive-gradle-plugin/   — Gradle plugin that builds Rust for Android targets (not a cargo member)
```

`ui-core` owns the platform-specific UI modules behind feature gates:

```
ui-core/src/widgets/      — Shared widget traits, modifiers, list diffing, Taffy bridge
ui-core/src/prop.rs       — Reactive prop descriptor
ui-core/src/encoding.rs   — Text offset conversions (UTF-16 ↔ codepoint)
ui-core/src/apple/        — Shared Apple helpers such as action targets
ui-core/src/appkit/       — macOS AppKit backend
ui-core/src/uikit/        — iOS UIKit backend
ui-core/src/android/      — Android JNI runtime and widget backend
ui-core/src/gtk/          — GTK backend
```

## Core Architecture (`core/`)

This is a fine-grained reactive component framework in Rust, inspired by SolidJS.

### Signal System

**Signal trait** (`signal/mod.rs`) — Object-safe trait with `read(&self) -> Self::Value`.

Implementations:
- **Primitive/String types** — implement `Signal` directly (no allocation)
- **`StoredSignal<T>`** (`signal/stored.rs`) — Mutable heap-allocated signal. Wraps `Rc<SignalInner<T>>` where `SignalInner` holds `RefCell<T>` + `WeakReactiveScope`. Pointer address is the stable `SignalId`. `update()` / `set_and_notify_changes()` / `update_if_changes()` mark dirty.
- **`ReadStoredSignal<T>`** — Read-only handle wrapping `StoredSignal<T>`; returned from memos, resources, and streams
- **`Fn() -> T`** — Any function is a computed signal (lazily re-evaluated)
- **`ConstantSignal<T>`** — Wrapper for constants; `into_signal()` converts any `Clone` type
- **`SignalExt`** (`signal/ext.rs`) — Adds `.map()` for transformation, `.boxed()` for type erasure

### ReactiveScope

`ReactiveScope` is `Rc<RefCell<ReactiveScopeData>>` (cheap to clone). Single owner of all reactive state.

**`ReactiveScopeData` fields**:
- `components: SlotMap<ComponentId, ComponentScope>` — Flattened tree
- `root: Vec<ComponentId>`
- `dirty_signals: DirtySignalSet` — Sorted dirty signal IDs + scheduler waker
- `active_signal_tracker: ActiveSignalTracker` — Automatic dependency capture

**Key methods**: `create_signal()`, `create_child_component()`, `dispose_component()`, `compare_components()`, `tick()`

### Component System

**`Component` trait** (`component.rs`):
- `fn setup(self: Box<Self>, ctx: &mut SetupContext)` — Consuming setup with object safety
- Blanket impls: `FnOnce(&mut SetupContext)` is a component; `()` is no-op; `Vec<BoxedComponent>` sets up each child

**`SetupContext`** — Wraps `(ReactiveScope, ComponentId)`. Public API for component setup. All methods take `&self`. Provides:
- `create_signal()`, `create_effect()`, `create_memo()`, `create_resource()`, `create_stream()`
- `create_fn_tracking()` → `FunctionTracker` — runs an arbitrary closure under dependency tracking and re-invokes a callback when those signals change. Platform layout code uses this to invalidate native views.
- `provide_context()`, `set_context()`, `set_static_context()`, `use_context()`
- `new_child()`, `child()`, `boxed_child()`, `component_id()`, `on_cleanup()`, `scope()`

Note the return types: `create_memo` / `create_resource` / `create_stream` return `ReadStoredSignal<T>`,
and `use_context` returns `Option<Rc<dyn Signal<Value = T>>>`.

**`ComponentScope`** (`component_scope.rs`) — Stored in the SlotMap:
- `parent: Option<ComponentId>`, `children: Vec<ComponentId>`
- `active_effects: Vec<Effect>` — Effects with dependencies or in-flight futures
- `inert_effects: Vec<BoxedEffectFn>` — Zero-dependency effects (run once, kept for cleanup)
- `cleanup: Vec<Box<dyn FnOnce()>>`
- `context: HashMap<ContextKeyId, BoxedStoredSignal>`

### Effects, Memos, Resources

**Effects** (`reactive_scope/effects.rs`):
- Closure: `FnMut(&ReactiveScope, Option<T>) -> T` (receives prior value)
- Dependencies auto-tracked via `ActiveSignalTracker`; runs immediately on creation

**Memos**: `create_memo(fn)` → `ReadStoredSignal<T>`. Derived cached signal, re-runs when dependencies change.

**Resources** (`reactive_scope/resources.rs`):
- `create_resource(input_signal, async_fn)` → `ReadStoredSignal<ResourceState<T>>`
- `ResourceState<T> = Loading(Option<T>) | Ready(T)` — carries last value during loads
- Input change during in-flight future immediately resets to `Loading`

**Streams**: `create_stream(initial, input, stream_fn)` → `ReadStoredSignal<T>`. Integrates `Stream<Item=T>`.

### Tick Loop (`reactive_scope/tick.rs`)

Two-phase execution to avoid nested borrows:

1. **Collect** — Traverse component tree DFS; partition effects into dirty/needs-poll/keep using dirty signal intersection
2. **Execute** — Run dirty effects with `&ReactiveScope` available; poll in-flight futures via custom `Waker`; push effects back

Effects are physically moved out of `ComponentScope.active_effects` via `extract_if` during tick, executed, then pushed back. No `Option`-wrapping needed.

**Waker**: Each future gets a `FutureWaker` that sets an `Arc<AtomicBool>` flag and calls the scheduler waker (e.g., GCD on macOS).

### Dependency Tracking (`reactive_scope/trackers.rs`)

**`DirtySignalSet`**: `Rc<RefCell<SortedVec<SignalId>>>` + waker. `mark_dirty(id)` adds ID and wakes scheduler.

**`ActiveSignalTracker`**: Stack of `SortedVec<SignalId>` tracking contexts. `on_accessed(id)` records accesses. `run_tracking(f)` push/run/pop, returns accessed signals.

**`SortedVec`** (`sorted_vec.rs`): Sorted for O(n+m) intersection. `intersects()` uses binary search for small sets, two-pointer merge otherwise.

### Context System (`reactive_scope/context.rs`)

- `ContextKey<T>` — Zero-size static type marker; pointer address is identity
- `provide_context(key, value)` → `StoredSignal<T>` — Stored in component's context HashMap
- `set_context(key, signal)` / `set_static_context(key, value)` — Store an existing signal or a constant
- `use_context(key)` → `Option<Rc<dyn Signal<Value = T>>>` — Walks parent chain

### Built-in Components (`components/`)

- **`Switch`** — Multi-branch conditional rendering; tracks `ActiveBranch = Case(usize) | Fallback`; disposes old branch on switch
- **`Show`** — Thin wrapper over Switch for boolean conditions
- **`Match`** (`match_component.rs`) — Pattern-matching over enum values. Per-case extractor `&mut T → Option<E>` (uses `std::mem::take`); per-case factory `ReadSignal<E> → BoxedComponent`. Rebuilds only on branch change; extracted value propagates through signal otherwise. `extract!` macro simplifies extractor syntax.
- **`For`** (`for_each.rs`) — Keyed list rendering. **Currently disabled**: both `mod for_each;` and
  `pub use for_each::For;` are commented out in `components/mod.rs`, and the file references an
  `EffectContext` type that does not exist in the crate. It will not compile as-is. See "Parked work".

## UI Core (`ui-core/`)

`ui-core` provides the shared widget API and all current platform backends. Platform modules are feature- and target-gated in `ui-core/src/lib.rs`.

### Shared Widgets (`ui-core/src/widgets/`)

- **Widget traits** — `Button`, `Label`, `TextInput`, `Image`, `ImageCodec`, `Slider`, `ProgressIndicator`, `Stack`, `Flex`, `Window`, `List`, and `Platform`
- **`Platform` trait** (`widgets/platform.rs`) — Associated type per widget plus `run_app()`,
  `native_view_registry_key()`, and `register_back_handler()`. `type List` is commented out; the
  list widget is not yet part of the platform surface.
- **`NativeView<BN, N>`** (`widgets/native.rs`) — Generic component wrapper. Creates a platform view, registers it with the nearest `NativeViewRegistry`, binds reactive props, and unregisters on cleanup.
- **`Prop<FrameworkType, Target, ValueType>`** (`prop.rs`) — Static prop descriptor wrapping a setter function. `bind()` creates an effect that applies the signal value to the native target.
- **Modifiers** — `Modifier` / `ModifierKey` plus common size and padding modifiers. `SizeSpec::Fixed` maps to Taffy `Dimension::length`; unspecified maps to `auto`.
- **Flex layout** — Shared `FlexProps` and item modifier keys (`flex_grow`, `flex_shrink`, `flex_basis`, `align_self`) feed into the Taffy-backed layout bridge. `CommonFlex<N>` and `CommonWindow<N>` are the shared container structs each backend specializes.
- **Lists** — `ListData`, `ListComparator`, `ListOrientation`, `List`, plus `DiffOp` / `DiffResult` / `diff()` in `list_diff.rs` provide reusable list snapshot/diff behavior for platform list views.
- **Encoding** (`ui-core/src/encoding.rs`, not under `widgets/`) — Helpers for platform text offset conversions: Apple/Android use UTF-16 code units; GTK uses Unicode codepoints.

### Taffy Layout (`ui-core/src/widgets/taffy.rs`)

`FlexTaffyContainer<N>` adapts native child views to Taffy:
- Stores root and child native views with `Modifier`, `ComponentId`, cached layout, and Taffy `NodeId`
- Maintains child order by comparing component positions in `ReactiveScope` via `compare_components()`
- Uses `compute_flexbox_layout` for the root and `compute_leaf_layout` for native leaves
- Delegates leaf measurement to the platform backend via `child_measurer`

### Apple Helpers (`ui-core/src/apple/`)

- **`ActionTarget`** — Custom NSObject subclass wrapping `Box<dyn Fn(&AnyObject)>`; attached via associated objects; implements action selector.

### AppKit Backend (`ui-core/src/appkit/`)

- **`appkit/ui/app_loop.rs`** — GCD `dispatch_async_f()` integration. `AppState` holds `ReactiveScope` + `AtomicBool tick_scheduled`; custom `Waker` routes signals to the macOS main queue.
- **`ReactiveFlexView`** (`appkit/ui/flex.rs`) — Owns a Taffy view tree and performs layout by setting native child frames directly. Implements `intrinsicContentSize` and `sizeThatFits:` by running Taffy measurements. A `FunctionTracker` invalidates intrinsic content size when tracked signals change.
- **Leaf measurement** — AppKit controls use `sizeThatFits(NSSize)`, other views use `fittingSize()`.
  `propose_size` maps `AvailableSpace::MinContent` to `0.5`, not `0`: AppKit treats a proposed `0` as
  "no constraint" rather than "narrowest possible". See "Parked work".
- **Widgets** — `Window`, `Button`, `Label`, `TextView`, `Stack`, `Flex`, `ImageView`, `CollectionView`, `ProgressIndicator`, and `Slider`.
- **View registry** — AppKit `Flex` provides a `NativeViewRegistry<Retained<NSView>>` context so child `NativeView`s can insert/remove themselves from the flex tree.

### UIKit Backend (`ui-core/src/uikit/`)

- iOS backend with `UIView`/`UIViewController` support and widgets for button, label, stack, text, and view controller integration.
- Uses the same shared widget traits and `NativeView`/registry pattern as other backends.
- Thinnest of the four backends — no flex, image, slider, or progress indicator yet.

### Android Backend (`ui-core/src/android/`, `android-lib/`, `android-macros/`)

- **JNI entrypoints** (`ui-core/src/android/mod.rs`) — `nativeCreate`, `nativeDestroy`, `nativeAttachActivity`, and `nativeTick`.
- **App loop** (`ui-core/src/android/app_loop.rs`) — Android waker integration; `nativeTick` clears `tick_scheduled`, builds a waker, and ticks the `ReactiveScope`.
- **Bindings/descriptors** (`ui-core/src/android/bindings.rs`, `desc.rs`) — Android class/method/property descriptors and generated binding support.
- **Widgets** (`ui-core/src/android/ui/`) — Button, label, flex/flex layout, image, list view, progress indicator, slider, stack, text input, window, listeners/watchers, and view component support.
- **`android-lib/`** — Kotlin wrapper exposing `ReactiveScope` to Android.
- **`android-macros/`** — Procedural macros for declaring JNI bindings.

### GTK Backend (`ui-core/src/gtk/`)

- GTK backend for macOS/Linux with widgets for button, flex, image view/codec, label, list view, progress indicator, slider, stack, text input, and window.
- Uses `NativeViewRegistry<gtk4::Widget>` and the shared widget traits.
- The only backend buildable on Linux, and the fastest way to exercise shared layout code.

## Android Build Support

- **`reactive-gradle-plugin/`** — Gradle plugin and tests for building Rust artifacts for Android ABIs.
- **`dexer/`** — DEX/class definition writer utilities.
- **`dexer-macros/`** — Validation and codegen macros for DEX generation.

## Examples

`ui-core/examples/flex_demo.rs` is the single runnable demo. It is generic over `Platform` and
dispatches to AppKit or GTK depending on the enabled feature. The old `demo/` and `demo-app/` crates
were removed when the workspace was consolidated.

## Parked Work

Known incomplete areas, so they are not mistaken for bugs introduced by a current change:

- **AppKit text line-breaking under Taffy** (`appkit/ui/flex.rs`) — The last work in the repo.
  Both a `preferredMaxLayoutWidth` measurement path and an Auto Layout min-size constraint path were
  tried and removed; what remains is the `MinContent => 0.5` workaround in `propose_size`. Not
  confirmed fixed.
- **Keyed list rendering** — `For` is disabled in `core`, and `Platform::List` is commented out,
  while backends already carry `list_view.rs` / `collection_view.rs`. This migration is unfinished.
- **`todo!()` sites** — GTK text input (`gtk/ui/text_input.rs`, 7 sites), AppKit `Stack` layout
  (`appkit/ui/stack.rs:47`), AppKit `register_back_handler` (`appkit/ui/platform.rs:43`). The GTK
  `register_back_handler` is an empty stub rather than a `todo!()`.
- **Stale doc comment** — The doc comment above `pub mod prop;` in `ui-core/src/lib.rs` describes
  logical layout components (`Padding`, `Center`, `Align`, `SizedBox`, `Expanded`) that do not exist;
  it belongs to a removed module.

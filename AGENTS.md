# AGENTS.md

This file provides guidance to Codex (Codex.ai/code) when working with code in this repository.

## Design Intent

Before making API-shaping or runtime-architecture changes, read [docs/architecture-principles.md](/Users/fanchao/Projects/reactive/docs/architecture-principles.md).

That document captures the intended direction for:
- SolidJS-style fine-grained reactivity without a virtual DOM
- shared cross-platform widget APIs
- first-class native-view escape hatches
- why the project should prefer direct native view ownership over a React Native-style shadow tree

## Build & Test Commands

```bash
cargo build                          # Build all
cargo test                           # Run all tests
cargo test <test_name>               # Run a single test
cargo test --lib                     # Library tests only
```

Format code with `cargo fmt` after changes.

## Workspace Structure

Rust workspace crates plus Android Gradle support:

```
core/                     — Core reactive framework (primary logic)
ui-core/                  — Cross-platform widget traits plus platform backends
resources/                — Runtime resource loading infrastructure
resources-build/          — Build-time resource utilities
reactive/                 — App-facing facade: re-exports, DefaultPlatform, prelude, `app!` entry macro
cargo-reactive/           — `cargo reactive` CLI: new/run/build/eject; Gradle and Xcode project templates
examples/flex-demo/       — Shared flex demo app (desktop, Android, iOS) built with `cargo reactive`
android-lib/              — Java support classes for the Android backend
android-macros/           — Android JNI binding codegen (not used by ui-core currently)
dexer/                    — DEX/class generation utilities (not used by ui-core currently)
dexer-macros/             — Macros for DEX/class generation
reactive-gradle-plugin/   — `com.reactive.android` Gradle plugin: cargo-ndk per variant, adds android-lib
```

`ui-core` owns the platform-specific UI modules behind feature gates:

```
ui-core/src/widgets/      — Shared widget traits, modifiers, list diffing, Taffy bridge
ui-core/src/apple/        — Shared Apple helpers such as action targets
ui-core/src/appkit/       — macOS AppKit backend
ui-core/src/uikit/        — iOS UIKit backend
ui-core/src/android/      — Android View-system backend (JNI)
ui-core/src/gtk/          — GTK backend
```

## Core Architecture (`core/`)

This is a fine-grained reactive component framework in Rust, inspired by SolidJS.

### Signal System

**Signal trait** (`signal/mod.rs`) — Object-safe trait with `read(&self) -> Self::Value`.

Implementations:
- **Primitive/String types** — implement `Signal` directly (no allocation)
- **`StoredSignal<T>`** (`signal/stored.rs`) — Mutable heap-allocated signal. Wraps `Rc<SignalInner<T>>` where `SignalInner` holds `RefCell<T>` + `WeakReactiveScope`. Pointer address is the stable `SignalId`. `update()` / `set_and_notify_changes()` / `update_if_changes()` mark dirty.
- **`ReadSignal<T>`** — Read-only handle wrapping `StoredSignal<T>`; returned from effects/resources/memos
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

**Key methods**: `create_signal()`, `create_child_component()`, `dispose_component()`, `tick()`, `setup_child()`

### Component System

**`Component` trait** (`component.rs`):
- `fn setup(self: Box<Self>, ctx: &mut SetupContext)` — Consuming setup with object safety
- Blanket impls: `FnOnce(&mut SetupContext)` is a component; `()` is no-op; `Vec<BoxedComponent>` sets up each child

**`SetupContext`** — Wraps `(ReactiveScope, ComponentId)`. Public API for component setup. All methods take `&self`. Provides:
- `create_signal()`, `create_effect()`, `create_memo()`, `create_resource()`, `create_stream()`
- `provide_context()`, `use_context()`
- `new_child()`, `child()`, `on_cleanup()`, `scope()`

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

**Memos**: `create_memo(ctx, fn)` → `ReadSignal<T>`. Derived cached signal, re-runs when dependencies change.

**Resources** (`reactive_scope/resources.rs`):
- `create_resource(ctx, input_signal, async_fn)` → `ReadSignal<ResourceState<T>>`
- `ResourceState<T> = Loading(Option<T>) | Ready(T)` — carries last value during loads
- Input change during in-flight future immediately resets to `Loading`

**Streams**: `create_stream(ctx, initial, input, stream_fn)` → `ReadSignal<T>`. Integrates `Stream<Item=T>`.

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
- `use_context(key)` → `Option<ReadSignal<T>>` — Walks parent chain

### Built-in Components (`components/`)

- **`Switch`** — Multi-branch conditional rendering; tracks `ActiveBranch = Case(usize) | Fallback`; disposes old branch on switch
- **`Show`** — Thin wrapper over Switch for boolean conditions
- **`Match`** (`match_component.rs`) — Pattern-matching over enum values. Per-case extractor `&mut T → Option<E>` (uses `std::mem::take`); per-case factory `ReadSignal<E> → BoxedComponent`. Rebuilds only on branch change; extracted value propagates through signal otherwise. `extract!` macro simplifies extractor syntax.

## UI Core (`ui-core/`)

`ui-core` provides the shared widget API and all current platform backends. Platform modules are feature- and target-gated in `ui-core/src/lib.rs`.

### Shared Widgets (`ui-core/src/widgets/`)

- **Widget traits** — `Button`, `Label`, `TextInput`, `Image`, `Slider`, `ProgressIndicator`, `Stack`, `Flex`, `Window`, and `Platform`
- **`NativeView<BN, N>`** (`widgets/native.rs`) — Generic component wrapper. Creates a platform view, registers it with the nearest `NativeViewRegistry`, binds reactive props, and unregisters on cleanup.
- **`Prop<FrameworkType, Target, ValueType>`** (`prop.rs`) — Static prop descriptor wrapping a setter function. `bind()` creates an effect that applies the signal value to the native target.
- **Modifiers** — `Modifier` / `ModifierKey` plus common size and padding modifiers. `SizeSpec::Fixed` maps to Taffy `Dimension::length`; unspecified maps to `auto`.
- **Flex layout** — Shared `FlexProps` and item modifier keys (`flex_grow`, `flex_shrink`, `flex_basis`, `align_self`) feed into the Taffy-backed layout bridge.
- **Lists** — `ListModel`, `DiffResult`, and `diff()` provide reusable list snapshot/diff behavior for platform list views.
- **Encoding** (`encoding.rs`) — Helpers for platform text offset conversions: Apple/Android use UTF-16 code units; GTK uses Unicode codepoints.

### Taffy Layout (`ui-core/src/widgets/taffy.rs`)

`FlexTaffyContainer<N>` adapts native child views to Taffy:
- Stores root and child native views with `Modifier`, `ComponentId`, cached layout, and Taffy `NodeId`
- Maintains child order by comparing component positions in `ReactiveScope`
- Uses `compute_flexbox_layout` for the root and `compute_leaf_layout` for native leaves
- Delegates leaf measurement to the platform backend via `child_measurer`

### Apple Helpers (`ui-core/src/apple/`)

- **`ActionTarget`** — Custom NSObject subclass wrapping `Box<dyn Fn(&AnyObject)>`; attached via associated objects; implements action selector.

### AppKit Backend (`ui-core/src/appkit/`)

- **`appkit/ui/app_loop.rs`** — GCD `dispatch_async_f()` integration. `AppState` holds `ReactiveScope` + `AtomicBool tick_scheduled`; custom `Waker` routes signals to the macOS main queue.
- **`ReactiveFlexView`** (`appkit/ui/flex.rs`) — Owns a Taffy view tree and performs layout by setting native child frames directly. Implements `intrinsicContentSize` and `sizeThatFits:` by running Taffy measurements.
- **Leaf measurement** — AppKit controls use `sizeThatFits(NSSize)`, other views use `fittingSize()`.
- **Widgets** — `Window`, `Button`, `Label`, `TextView`, `Stack`, `Flex`, `ImageView`, `ProgressIndicator`, and `Slider`.
- **View registry** — AppKit `Flex` provides a `NativeViewRegistry<Retained<NSView>>` context so child `NativeView`s can insert/remove themselves from the flex tree.

### UIKit Backend (`ui-core/src/uikit/`)

- `ui_core::uikit::platform::UIKit` implements `Platform`: Label, Button, Image, Slider, ProgressIndicator, Flex (Taffy in a `UIView` subclass) and Window are real; Stack and TextInput are `Unsupported` placeholders.
- Scene lifecycle: `run_app`/`run_app_reconnectable` call `UIApplicationMain` with Rust-defined `ReactiveAppDelegate` and `ReactiveSceneDelegate`; setup runs per connected scene, which is provided via the `WINDOW_SCENE` context key. The Info.plist needs the `UIApplicationSceneManifest` that `cargo reactive` generates.
- The GCD tick loop in `ui-core/src/apple/app_loop.rs` is shared with AppKit.
- Type-checked only (`cargo check --target aarch64-apple-ios`); not yet run on a device or simulator.
- Uses the same shared widget traits and `NativeView`/registry pattern as other backends.

### Android Backend (`ui-core/src/android/`, `android-lib/`)

Classic `android.view.View` backend; Rust owns real views via JNI (`jni` crate, no codegen). Design, findings and TODOs: [docs/android.md](docs/android.md).

- **Entry point** — `ui_core::android_main!(setup_fn)` expands to `JNI_OnLoad`, which registers native methods and stores the setup fn. `com.reactive.ReactiveActivity` (named by `com.reactive.lib_name` manifest meta-data) calls `nativeCreate`/`nativeDestroy`; the activity is provided via the `ACTIVITY` context key.
- **Tick loop** (`app.rs`) — a pipe registered on the main thread's `ALooper`; wakers (any thread) write one byte, the looper callback ticks the scope inside a JNI local frame.
- **`java.rs`** — `JavaObject` (`GlobalRef` wrapper) and call helpers; Java exceptions are described and cleared before panicking. Panics are logged to logcat (tag `reactive`).
- **Widgets** (`ui/`) — `Label` (TextView), `Image` (ImageView, `BitmapCodec` via `BitmapFactory`), `Button` (click via `NativeCallback`), `Flex`, `Window` (activity content `FrameLayout` with `fitsSystemWindows`, title). ProgressIndicator/Slider/Stack/TextInput are `Unsupported` placeholders.
- **Custom layouts** — `com.reactive.ReactiveLayout` (ViewGroup) forwards `onMeasure`/`onLayout` to any Rust `ViewGroupLayout` installed with `layout::attach` (`ui-core/src/android/ui/layout.rs`).
- **Flex** — a `ViewGroupLayout` that runs `FlexTaffyContainer` in dp and measures children with `View.measure` (min-content approximated as `AT_MOST 0`).
- **`android-lib/`** — Java only (`ReactiveActivity`, `ReactiveLayout`, `NativeCallback`); no Kotlin plugin needed.
- **Demo** — `cargo reactive run --target android -p flex-demo`; requires `cargo-ndk`. See docs/getting-started.md.

### GTK Backend (`ui-core/src/gtk/`)

- GTK backend for macOS/Linux with widgets for button, flex, image view/codec, label, list view, progress indicator, slider, stack, text input, and window.
- Uses `NativeViewRegistry<gtk4::Widget>` and the shared widget traits.

## Android Build Support

- **`reactive-gradle-plugin/`** — `com.reactive.android` plugin (`reactive { crateDir; packageName }`); `gradlew -p reactive-gradle-plugin test publishToMavenLocal`.
- **`dexer/`** — DEX/class definition writer utilities.
- **`dexer-macros/`** — Validation and codegen macros for DEX generation.

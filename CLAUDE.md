# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
cargo build                          # Build all
cargo test                           # Run all tests
cargo test <test_name>               # Run a single test
cargo test --lib                     # Library tests only
```

Format code with `cargo fmt` after changes.

## Workspace Structure

13 crates organized by platform and functionality:

```
core/              — Core reactive framework (primary logic)
ui-utils/          — Cross-platform UI utility patterns (RecyclingList)
apple/             — macOS/iOS FFI, prop bindings, action targets, GCD app loop
appkit/            — macOS AppKit widget bindings
uikit/             — iOS UIKit bindings
android/           — Android JNI entrypoints
android-macros/    — Android prop descriptor codegen
android-lib/       — Android library support
ui/                — Legacy UI module
resources/         — Resource loading infrastructure
resources-build/   — Build-time resource utilities
demo/              — macOS demo application
demo-app/          — Android demo application
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

## Platform Bindings

### Apple Platform (`apple/`)

- **`AppLoop`** — GCD `dispatch_async_f()` integration. `AppState` holds `ReactiveScope` + `AtomicBool tick_scheduled`. Custom `Waker` routes signals to GCD main queue.
- **`Prop<RustType, ObjCType, ObjCValueType>`** — Static descriptor for a setter (fn pointer). `bind(ctx, view, signal)` creates an effect calling the setter on changes.
- **`ViewBuilder`** — Stores creator closure + property binder closures; `setup(ctx)` creates view and applies all binders.
- **`ActionTarget`** — Custom NSObject subclass wrapping `Box<dyn Fn(&AnyObject)>`; attached via associated objects; implements action selector.
- **`view_props!` macro** — Generates static `Prop` descriptors; handles `String → NSString`; generates camelCase setter names.

### AppKit Widgets (`appkit/src/ui/`)

- **`Window`** — NSWindow wrapper with delegate for close; sets content view as context parent
- **`Button`** — Props: `title`, `enabled`, `highlighted`; takes `on_click` callback
- **`TextView`** (`text_view.rs`) — Custom `ReactiveTextStorage` (NSTextStorage subclass). Implements TextKit primitives. `label()` mode (read-only) or `TextInputState` mode (editable, with UTF-16 selection). Notifies signal on edit commit.
- **`Stack`** — NSStackView wrapper; propagates `ViewParent` context
- **`CollectionView`** — Cell recycling pattern. Create path: allocate `StoredSignal<Item>`, call `on_create`, setup component, return opaque token. Reuse path: update signal (no-op if unchanged). Cleanup via component disposal.
- **`Checkbox`**, **`ImageView`**, **`ProgressIndicator`**, **`Slider`** — Standard prop-based widgets
- **`AppKitViewComponent<V, Children>`** — Generic wrapper; converts to `NSView`, adds to parent from context; supports `Vec`, `Option`, or `()` children

**`ViewParent` context** (`appkit/src/ui/context.rs`) — `Window | Stack`; `add_child()` / `remove_child()` polymorphic operations.

### Android (`android/`, `android-macros/`)

JNI entrypoints: `nativeCreate`, `nativeDestroy`, `nativeTick` (uses `Waker::noop()`).
`PropDescriptor` — static descriptors with JNI class/method/signature strings.
`view_props!` macro generates `PropDescriptor` instances with Rust→JNI type mapping.

## UI Utilities (`ui-utils/`)

**`RecyclingList<Item>`** (`ui-utils/src/recycling_list.rs`) — Cross-platform recycling/collection view pattern:
- `item_count()` — Current snapshot size
- `create_cell(scope, parent, index, on_create)` → `*const ()` opaque token — Allocates per-cell `StoredSignal<Item>`, builds component tree, registers cleanup
- `reuse_cell(token, index)` — Recovers signal via pointer, updates value (reactive if changed)
- Reconciliation effect snapshots items on every change, invokes `on_reload()` callback for platform refresh

Used by `CollectionView` in AppKit; designed for reuse in UIKit/Android.

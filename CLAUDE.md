# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
cargo build                          # Build all
cargo test                           # Run all tests
cargo test <test_name>               # Run a single test
cargo test --lib                     # Library tests only
```

Generally you should format code with `cargo fmt` after changes.

## Architecture

This is a fine-grained reactive component framework in Rust, inspired by SolidJS. The single workspace member is `core`.

### Reactivity Model

**ReactiveScope** is the single global owner of all state. It holds:
- Signals in a `SlotMap<SignalId, Box<dyn Any>>` — type-erased, accessed via typed `Signal<T>` handles
- Components in a `SlotMap<ComponentId, ComponentScope>` — a flattened tree (parent/children are key references, not nested ownership)
- Dirty signal tracking via `SortedVec<SignalId>` for O(n+m) intersection checks

**Tick loop** (`ReactiveScope::tick`): collects dirty effects/resources per component using `extract_if` (swap-to-back partition), runs them with `&mut self` available, then restores them. Futures are polled afterward; completed futures mark their signal dirty for the next tick.

### Borrow Checker Patterns

Effects and resources are stored directly in `ComponentScope` (not in global slotmaps). During tick, they are physically moved out of the component's `Vec` via `extract_if`, executed with full `&mut ReactiveScope` access, then pushed back. This avoids `Option`-wrapping or placeholder patterns.

### Component System

- `Component` trait uses `self: Box<Self>` for consuming setup with object safety
- `SetupContext<'a>` wraps `(&mut ReactiveScope, ComponentId)` — the public API surface for component setup
- `EffectContext<'a>` wraps `(&mut ReactiveScope, &mut SortedVec<SignalId>)` — tracks signal dependencies automatically
- Blanket impl: any `FnOnce(&mut SetupContext)` is a `Component`
- `()` is a no-op component

### Context

Context uses `Rc<HashMap<ContextKeyId, SignalId>>` with copy-on-write via `Rc::make_mut`. Children inherit the parent's context Rc on creation; `provide_context` clones the map only for the providing component's subtree.

### Built-in Components (`components/`)

- **Switch**: Multi-branch conditional rendering with `ActiveBranch` enum tracking
- **Show**: Thin wrapper over Switch for boolean conditions (takes both `then` and `otherwise`)
- **For**: Keyed list reconciliation — reuses children by key, disposes removed ones

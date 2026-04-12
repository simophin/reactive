# Architecture Principles

This document captures the intended architectural direction for the reactive UI framework in this repository. It is aimed at contributors and agents making API or runtime decisions.

## Goals

The framework is guided by three primary goals:

1. SolidJS-style execution model
   Reactive updates should flow directly to real UI objects through fine-grained reactivity. The framework should not depend on a virtual DOM or a React Native-style shadow tree diff.
2. Cross-platform widget vocabulary
   Common widgets should be exposed through a shared API across platforms, covering the common use cases without hiding platform capabilities.
3. Native escape hatch
   It must remain possible to drop down to native views and native layout behavior because the framework operates in the real native layout pass.

## Architectural Invariants

These principles should hold unless there is a deliberate design change:

- Public visual widgets should correspond to real native views or native container views.
- Reactive updates should target native views directly instead of diffing an intermediate UI tree.
- Layout should be derived from parent-child relationships in the real component/native view hierarchy, not from a detached layout tree.
- Cross-platform abstractions should focus on the common 80 percent case and avoid blocking platform-specific customization.
- Native interop is a first-class capability, not an afterthought or an escape from the "real" framework.

## Non-Goals

The framework is not trying to become:

- A virtual-DOM renderer.
- A React Native-style system with a separate shadow layout tree.
- A full Compose clone with arbitrary rendering, gesture, and semantics modifiers on every node.
- A framework where every logical concept must be represented as a visible native view.

## What We Borrow From Other Systems

### SolidJS

SolidJS is the primary architectural inspiration:

- fine-grained reactive updates
- explicit dataflow
- direct ownership of live UI objects
- minimal hidden runtime structure

This is the strongest philosophical constraint on the design.

### Flutter

Flutter is a useful reference for parent-driven layout semantics:

- parent offers constraints
- child measures within those constraints
- parent decides final placement

These semantics are compatible with this framework even if the public API does not resemble Flutter's wrapper-heavy widget style.

### Jetpack Compose

Compose is primarily an API ergonomics reference, not a runtime model reference.

Compose-style modifier syntax may be a good fit when it compiles down to plain layout or view metadata on a child. That is especially true for:

- padding
- alignment
- sizing
- flex or weight-style parent data

Compose is a weaker fit when a modifier would require hidden wrapper views, a separate rendering subsystem, or generic gesture/drawing infrastructure on every node.

### React Native

React Native is a useful comparison point, but not the target architecture.

Its shared widget vocabulary is relevant, but its shadow-tree-plus-Yoga layout architecture is not aligned with the goals above. If choosing between direct native view ownership and a detached layout tree, prefer direct native view ownership.

## Implications For Public API Design

### Widget Model

The preferred model is:

- visible widgets map 1:1 to native views or native container views
- layout-only concepts should avoid becoming standalone public widgets when they can be represented as metadata
- logical components remain valid for control flow, composition, and reactive orchestration

Examples of logical components that still make sense:

- `Show`
- `Switch`
- `Match`
- async/resource composition helpers
- list/recycling abstractions

### Layout API

The runtime layout principle does not need to change if the public API changes.

It is acceptable to keep a parent-driven layout model while exposing a more Compose-like surface. For example:

- current wrapper form: `Padding { child: ... }`
- possible future form: `child.modifier(Modifier::new().padding(...))`

These are different API surfaces over the same core idea: child layout metadata consumed by the parent during measurement and placement.

### Modifier Direction

Modifier-style APIs are desirable only when they lower directly into existing metadata or parent-data concepts.

Good candidates:

- padding
- align
- width / height / size
- flex / weight / expand

High-risk candidates:

- arbitrary background drawing
- generic clickable / gesture modifiers
- clipping, overlays, or visual effects that require additional rendering layers

Those higher-risk APIs should not be adopted casually if they compromise the direct-native-view model.

## Preferred Mental Model

The intended mental model is:

- the component tree is real
- visible widgets own real native views
- signals update those views directly
- parents lay out children using explicit reactive metadata
- native interop participates in the same system as built-in widgets

The framework should feel closer to "SolidJS for native views" than to "React Native in Rust".

## Guidance For Future Changes

When evaluating a new abstraction, ask:

1. Does this preserve direct fine-grained updates to real native views?
2. Does this avoid introducing a hidden intermediate tree?
3. Does this improve the shared cross-platform API without blocking native escape hatches?
4. Does this abstraction compile down to metadata, or does it secretly require a new runtime subsystem?
5. Does this make native layout participation clearer or more obscure?

If a proposal improves syntax but weakens those properties, it is likely moving away from the framework's intended direction.

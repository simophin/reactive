# Android Backend (View system)

Status: first iteration, captured 2026-10-03. Covers Text, Image, Button, Flex and Window.

The backend drives the classic `android.view.View` system from Rust through JNI. Rust creates and owns real views, and signals call their setters directly. This follows [architecture-principles.md](./architecture-principles.md). An earlier Compose design was considered and archived: [archive/android-compose-research.md](./archive/android-compose-research.md).

## Layout of the code

| Where | What |
|---|---|
| `ui-core/src/android/app.rs` | `JNI_OnLoad` handling, activity create/destroy, tick loop, panic hook |
| `ui-core/src/android/java.rs` | `JavaObject` (`GlobalRef` wrapper) and JNI call helpers |
| `ui-core/src/android/callback.rs` | `NativeCallback` trampoline: Java listener → Rust closure |
| `ui-core/src/android/ui/` | Widgets: `label`, `image`, `button`, `flex`, `window`, `unsupported`, `platform`; `layout` for custom `ViewGroup` layouts |
| `android-lib/` | Java support classes: `ReactiveActivity`, `ReactiveLayout`, `NativeCallback` |
| `examples/flex-demo/` | Shared demo crate (desktop bin plus Android cdylib); Gradle app in `android/` |

Run the demo with `cd examples/flex-demo/android && ./gradlew installDebug`. Add `-Preactive.abis=arm64-v8a,x86_64` when targeting an emulator. It needs `cargo-ndk` and an Android SDK/NDK.

## Design

**Entry point.**
- The app crate calls `ui_core::android_main!(setup_fn)`, which expands to `JNI_OnLoad`.
- `JNI_OnLoad` stores the setup function and registers every native method with `RegisterNatives`.
- Defining the only exported symbol in the app crate avoids relying on `#[no_mangle]` symbols from an rlib surviving into the cdylib.
- `ReactiveActivity` finds the library name in its `com.reactive.lib_name` manifest meta-data, the same way `NativeActivity` does. App code does not subclass anything.
- The activity is passed down the tree through the `ACTIVITY` context key, not stored in a global.

**Tick loop.**
- Each activity registers a pipe on the main thread's `ALooper`, the native side of `Handler`/`MessageQueue`.
- `TickSignal` implements `std::task::Wake`. Waking swaps a `scheduled` flag and writes one byte, so wakers work from any thread (for example tokio workers) and never touch the JVM.
- The looper callback reads the byte, clears the flag and ticks the scope inside `with_local_frame`. The callback is not a JNI call, so local references would otherwise leak.
- This matches the GCD (`dispatch_async_f`) and glib (`idle_add`) loops on other platforms.

**Views.**
- Each widget is a `NativeView<JavaObject, X>`. `X` is a typed newtype (`TextView`, `ImageView`, …) so each widget is a distinct type, as in the GTK backend.
- Props are plain `Prop` setters calling methods by name and signature.
- `JavaObject` equality compares references. Clones of one `GlobalRef` compare equal, which is all the view registries need.

**Custom layouts.**
- `ReactiveLayout` is a generic `ViewGroup`. Its `onMeasure` and `onLayout` call `nativeMeasure` and `nativeLayout` with a handle held in a Java field. The class has no layout logic of its own.
- On the Rust side, any container implements `ViewGroupLayout` (`measure` from two measure specs, `layout` into a width and height, all in px) and calls `layout::attach` to install it on a view from `new_layout_view`. The handle points to a boxed `Rc<dyn ViewGroupLayout>` and is cleared on cleanup.
- `layout.rs` also has the `MeasureSpec` helpers, `resolve_size`, and `measure_view`/`layout_view` for children.

**Flex.**
- Flex is one `ViewGroupLayout` over a `ReactiveLayout`.
- Rust runs the shared `FlexTaffyContainer`, the same one GTK and AppKit use.
- Taffy works in dp. Conversion to and from px happens only at this boundary, so modifier values mean the same on every platform.
- Children are measured with `View.measure`:
  - a known size becomes `EXACTLY`
  - a definite available size becomes `AT_MOST`
  - max-content becomes `UNSPECIFIED`. This means "no constraint": the view reports its natural size. For most leaf widgets that matches max-content (text lays out on a single line), but that's a consequence of how each view measures itself, not part of the spec's meaning.
  - min-content becomes `AT_MOST 0` (see below)
- `onLayout` collects placements first, then re-measures each child with `EXACTLY` at its final size before calling `layout`. TextView builds its text layout during measure.
- Edges are rounded rather than sizes, so adjacent children never gap or overlap.

**Window.**
- The activity's content view is a `FrameLayout` with `fitsSystemWindows`. It keeps the tree clear of system bars under the edge-to-edge mode enforced from API 35.
- Its default `MATCH_PARENT` layout params give the root Flex the full content area.

**Java, not Kotlin.** The support classes are plain Java. AGP 9's built-in Kotlin pulls in `kotlin-stdlib` and needs extra configuration, while Java needs no plugin at all.

## Findings against the architecture

Measured against [architecture-principles.md](./architecture-principles.md):

- **Direct native ownership holds.** Every visible widget is a real `View` that Rust creates and holds as a `GlobalRef`. Signals call its setters synchronously, with no intermediate tree or serialization. The Java side contains no widget logic: it only forwards lifecycle, layout and listener calls.
- **Layout runs in the real native pass.** Flex takes part in Android's own `measure`/`layout` traversal through a `ViewGroup` subclass, and nests with any other `View`. Native widgets measure themselves, and Taffy only arranges them. The shared Taffy container ran unchanged; only the measure bridge and the px/dp conversion are Android-specific.
- **The native parent-driven model maps cleanly, with one gap.** Android's `MeasureSpec` covers known sizes (`EXACTLY`) and definite available space (`AT_MOST`). `UNSPECIFIED` asks for the view's natural size, which stands in for max-content: for text and most leaf widgets the two match, since nothing forces a wrap. `MeasureSpec` has no min-content query. Offering `AT_MOST 0` makes views report their minimum (a button's `minWidth`; zero for text), which behaves like CSS `min-width: 0`. Using the natural size instead would stop wrapping text from ever shrinking.
- **Layout units need one boundary.** Modifier values are platform-neutral numbers. Treating them as dp and converting only inside the Flex bridge keeps them consistent with points on Apple and logical pixels on GTK.
- **Modifier changes are not yet reactive for layout.** Prop setters such as `setText` invalidate layout natively through `requestLayout`. Modifier signals are only read during layout, so changing one doesn't trigger a relayout. This applies to every backend, not just Android.
- **The window concept maps to the activity.** `Window` becomes the activity's content view and title; the initial size has no meaning. Edge-to-edge (enforced from API 35) means the window must keep content clear of system bars, done here with `fitsSystemWindows`.
- **Activity lifetime is shorter than app lifetime.** Configuration changes recreate the activity, and the reactive scope currently lives and dies with it. Keeping state across recreation is an open design question (see below). The demo sidesteps it with `configChanges`.
- **Async integrates without touching the JVM.** Wakers from any thread only write to a pipe watched by the main `ALooper`. The pipe's ends must outlive every waker, or a late wake raises `SIGPIPE`. Both ends live in the shared `TickSignal` for that reason.
- **JNI cost is per call.** Each prop update or child measure is one or more JNI calls with a method lookup by name. That was fine for this demo but is the obvious cost centre (see below).
- **The shared widget vocabulary has gaps.** `Image` has no `WithModifier`, so it can't be sized from a parent Flex on any platform.

## Verified

- Samsung A52 (arm64): launch, layout, Show/Hide text, Add gap.
- API 36 x86_64 emulator, edge-to-edge enforced:
  - portrait and landscape relayout
  - six destroy/recreate cycles in one process via `--activity-clear-task`
  - cross-thread wakes from tokio timers, tested with a temporary 300 ms timer
- Clippy is clean for Android and GTK, and the workspace tests pass.

## Left to do

**Widgets**
- [ ] ProgressIndicator (`ProgressBar`), Slider (`SeekBar`) and Stack (`FrameLayout` plus alignment). These are `Unsupported` placeholders today.
- [ ] TextInput (`EditText`) with UTF-16 selection and a `TextWatcher` through `NativeCallback`.
- [ ] Lists (`RecyclerView`) driven by the shared list diffing.
- [ ] `Image` sizing: add `WithModifier` to the shared `Image` trait.

**Runtime**
- [ ] Activity recreation: without `configChanges` a rotation rebuilds the tree and loses state. Decide whether state should survive (for example a retained scope) or document `configChanges` as required.
- [ ] Back handling: `Platform::register_back_handler` is a no-op. Wire it to `OnBackPressedDispatcher` or `OnBackInvokedCallback`.
- [ ] Embedding: a `ReactiveView`/host that can live inside an existing Activity, Fragment or Compose `AndroidView`, not just a dedicated activity.
- [ ] Logging: route `tracing` to logcat. Only panics are logged today.

**Layout and performance**
- [ ] Cache `jmethodID`s. Every call currently looks the method up by name and signature.
- [ ] Batch child measurement if JNI overhead shows up in profiles; each child measure is about three calls.
- [ ] Changes to modifier signals don't trigger a relayout on their own; this is a gap shared with the other backends.
- [ ] Child z-order and accessibility order follow insertion order, not component order.

**Build**
- [ ] Fix or remove `reactive-gradle-plugin`: it uses `Project.exec`, which Gradle 9 removed. The demo uses its own `cargo ndk` task instead. Also decide whether `android-lib` is published or always consumed via `includeBuild`.
- [ ] Decide the fate of `dexer`, `dexer-macros` and `android-macros`, which `ui-core` no longer uses.
- [ ] Run the Android demo build in CI.

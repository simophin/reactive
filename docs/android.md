# Android Backend (View system)

Status: first iteration, captured 2026-10-03. Covers Text, Image, Button, Flex and Window.

The backend drives the classic `android.view.View` system from Rust through JNI. Rust creates and owns real views, and signals call their setters directly. This follows [architecture-principles.md](./architecture-principles.md). An earlier Compose design was considered and archived: [archive/android-compose-research.md](./archive/android-compose-research.md).

## Layout of the code

| Where | What |
|---|---|
| `ui-core/src/android/app.rs` | `JNI_OnLoad` handling, activity create/destroy, tick loop, panic hook |
| `ui-core/src/android/java.rs` | `JavaObject` (`GlobalRef` wrapper) and JNI call helpers |
| `ui-core/src/android/callback.rs` | `NativeCallback` trampoline: Java listener → Rust closure |
| `ui-core/src/android/ui/` | Widgets: `label`, `image`, `button`, `flex`, `window`, `unsupported`, `platform` |
| `android-lib/` | Java support classes: `ReactiveActivity`, `ReactiveFlexLayout`, `NativeCallback` |
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

**Flex.**
- `ReactiveFlexLayout` is a `ViewGroup`. Its `onMeasure` and `onLayout` call `nativeMeasure` and `nativeLayout` with a handle (an `Rc<FlexState>` pointer) held in a Java field.
- Rust runs the shared `FlexTaffyContainer`, the same one GTK and AppKit use.
- Taffy works in dp. Conversion to and from px happens only at this boundary, so modifier values mean the same on every platform.
- Children are measured with `View.measure`:
  - a known size becomes `EXACTLY`
  - a definite available size becomes `AT_MOST`
  - max-content becomes `UNSPECIFIED`
  - min-content becomes `AT_MOST 0` (see below)
- `onLayout` collects placements first, then re-measures each child with `EXACTLY` at its final size before calling `layout`. TextView builds its text layout during measure.
- Edges are rounded rather than sizes, so adjacent children never gap or overlap.

**Window.**
- The activity's content view is a `FrameLayout` with `fitsSystemWindows`. It keeps the tree clear of system bars under the edge-to-edge mode enforced from API 35.
- Its default `MATCH_PARENT` layout params give the root Flex the full content area.

**Java, not Kotlin.** The support classes are plain Java. AGP 9's built-in Kotlin pulls in `kotlin-stdlib` and needs extra configuration, while Java needs no plugin at all.

## Discoveries

- **The old backend was dead code.** `ui-core/src/android/` targeted an older API (`ui_core::layout`, `PlatformViewBuilder`, `Row`/`Column`) and failed to compile with 76 errors.
- **The old waker was broken.** It called `java_vm.get_env()` to post a tick. That fails on threads not attached to the JVM, which is exactly where async runtimes wake from. The looper pipe avoids JNI on wake entirely.
- **Pipe lifetime matters.** Writing to a pipe whose read end is closed raises `SIGPIPE`. Both ends live in the shared `TickSignal`, so a waker that outlives the activity writes into an unread pipe instead.
- **Views have no min-content query.** Offering `AT_MOST 0` makes views report their minimum (a button's `minWidth`; zero for text). This matches CSS `min-width: 0` closely enough for flex shrinking. Using the natural size instead stops wrapping text from ever shrinking.
- **Gradle 9 removed `Project.exec`.** `reactive-gradle-plugin` uses it, so the plugin is likely broken. The demo uses a ~30-line `cargo ndk` task built on `ExecOperations` instead.
- **AGP 9 needs repositories in included builds.** It resolves `aapt2` per build, so `android-lib` needs its own `dependencyResolutionManagement` repositories when pulled in via `includeBuild`.
- **`ui-core` no longer needs to be a cdylib.** Built as one, it shipped an unused `libui_core.so` in the APK. The app crate is the only cdylib now.

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
- [ ] `Image` sizing: the shared `Image` trait has no `WithModifier`, so images can't be sized from a Flex.

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
- [ ] Fix or remove `reactive-gradle-plugin` for Gradle 9, and decide whether `android-lib` is published or always consumed via `includeBuild`.
- [ ] Decide the fate of `dexer`, `dexer-macros` and `android-macros`, which `ui-core` no longer uses.
- [ ] Run the Android demo build in CI.

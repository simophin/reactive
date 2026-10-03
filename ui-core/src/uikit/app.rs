//! App and scene lifecycle.
//!
//! [`run_app`] stores the setup function and calls `UIApplicationMain` with
//! `ReactiveAppDelegate`. Every connecting `UIWindowScene` gets its own
//! [`ReactiveScope`], built by the setup function in
//! `scene:willConnectToSession:options:` and disposed in `sceneDidDisconnect:`.
//! Ticks run on the GCD main queue (see `crate::apple::app_loop`).

use super::ui::WINDOW_SCENE;
use crate::apple::app_loop::AppLoop;
use objc2::rc::{Allocated, Retained};
use objc2::runtime::AnyObject;
use objc2::{
    ClassType, DefinedClass, MainThreadMarker, MainThreadOnly, Message, define_class, msg_send,
};
use objc2_foundation::{NSObject, NSObjectProtocol, NSString};
use objc2_ui_kit::{
    UIApplication, UIApplicationDelegate, UIResponder, UIScene, UISceneConfiguration,
    UISceneConnectionOptions, UISceneDelegate, UISceneSession, UIWindowScene,
    UIWindowSceneDelegate,
};
use reactive_core::{ComponentId, ReactiveScope, SetupContext};
use std::cell::RefCell;
use std::rc::Rc;

/// Name of the scene configuration returned by `ReactiveAppDelegate`; matches
/// the one the generated Info.plist declares.
const SCENE_CONFIGURATION_NAME: &str = "Default Configuration";

type BoxedSetup = Box<dyn FnOnce(&mut SetupContext)>;

enum AppSetup {
    /// Runs for the first scene only.
    Once(Option<BoxedSetup>),
    /// Runs for every scene that connects.
    Reconnectable(Rc<dyn Fn(&mut SetupContext)>),
}

thread_local! {
    static APP_SETUP: RefCell<Option<AppSetup>> = const { RefCell::new(None) };
}

fn take_setup() -> Option<BoxedSetup> {
    APP_SETUP.with(|setup| match setup.borrow_mut().as_mut()? {
        AppSetup::Once(setup) => setup.take(),
        AppSetup::Reconnectable(setup) => {
            let setup = setup.clone();
            let setup: BoxedSetup = Box::new(move |ctx| setup(ctx));
            Some(setup)
        }
    })
}

/// Starts UIKit and never returns. Must be called on the main thread, e.g.
/// from an `extern "C"` function that the Xcode host's `main()` calls.
///
/// `setup` builds the component tree for the first scene that connects. It is
/// `FnOnce`, so if iOS later disconnects that scene (it may do so to reclaim
/// memory while the app is in the background) and connects a new one, there
/// is nothing to rebuild it with and the new scene stays empty. Use
/// [`run_app_reconnectable`] to rebuild the tree for every scene.
pub fn run_app(setup: impl FnOnce(&mut SetupContext) + 'static) -> ! {
    start(AppSetup::Once(Some(Box::new(setup))))
}

/// Like [`run_app`], but runs `setup` again for every scene that connects,
/// so the UI is rebuilt after iOS disconnects and reconnects the scene.
pub fn run_app_reconnectable(setup: impl Fn(&mut SetupContext) + 'static) -> ! {
    start(AppSetup::Reconnectable(Rc::new(setup)))
}

fn start(setup: AppSetup) -> ! {
    let mtm = MainThreadMarker::new().expect("run_app must be called on the main thread");
    APP_SETUP.with(|slot| *slot.borrow_mut() = Some(setup));

    // UIKit looks both classes up by name, so register them first.
    let _ = ReactiveSceneDelegate::class();
    let delegate_class = NSString::from_class(ReactiveAppDelegate::class());
    UIApplication::main(None, Some(&delegate_class), mtm)
}

define_class!(
    // SAFETY: UIResponder has no subclassing requirements and this class does
    // not implement Drop.
    #[unsafe(super(UIResponder, NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "ReactiveAppDelegate"]
    struct ReactiveAppDelegate;

    unsafe impl NSObjectProtocol for ReactiveAppDelegate {}

    unsafe impl UIApplicationDelegate for ReactiveAppDelegate {
        #[unsafe(method(application:didFinishLaunchingWithOptions:))]
        fn did_finish_launching(
            &self,
            _application: &UIApplication,
            _launch_options: Option<&AnyObject>,
        ) -> bool {
            true
        }

        /// Hands every new session to `ReactiveSceneDelegate`, so the scene
        /// delegate is used even if the Info.plist names no delegate class.
        #[unsafe(method_id(application:configurationForConnectingSceneSession:options:))]
        fn configuration_for_connecting_scene_session(
            &self,
            _application: &UIApplication,
            session: &UISceneSession,
            _options: &UISceneConnectionOptions,
        ) -> Retained<UISceneConfiguration> {
            let configuration = UISceneConfiguration::configurationWithName_sessionRole(
                Some(&NSString::from_str(SCENE_CONFIGURATION_NAME)),
                &session.role(),
                self.mtm(),
            );
            // SAFETY: the class is a UIResponder conforming to
            // UIWindowSceneDelegate, as UIKit requires.
            unsafe { configuration.setDelegateClass(Some(ReactiveSceneDelegate::class())) };
            configuration
        }
    }
);

struct SceneState {
    app_loop: AppLoop,
    root: ComponentId,
}

define_class!(
    // SAFETY: UIResponder has no subclassing requirements and this class does
    // not implement Drop.
    #[unsafe(super(UIResponder, NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "ReactiveSceneDelegate"]
    #[ivars = RefCell<Option<SceneState>>]
    struct ReactiveSceneDelegate;

    impl ReactiveSceneDelegate {
        // UIKit creates the delegate with `alloc`/`init`; the ivars must be
        // initialised here.
        #[unsafe(method_id(init))]
        fn init(this: Allocated<Self>) -> Retained<Self> {
            let this = this.set_ivars(RefCell::new(None));
            unsafe { msg_send![super(this), init] }
        }
    }

    unsafe impl NSObjectProtocol for ReactiveSceneDelegate {}

    unsafe impl UISceneDelegate for ReactiveSceneDelegate {
        #[unsafe(method(scene:willConnectToSession:options:))]
        fn scene_will_connect(
            &self,
            scene: &UIScene,
            _session: &UISceneSession,
            _options: &UISceneConnectionOptions,
        ) {
            self.connect(scene);
        }

        #[unsafe(method(sceneDidDisconnect:))]
        fn scene_did_disconnect(&self, _scene: &UIScene) {
            self.disconnect();
        }
    }

    unsafe impl UIWindowSceneDelegate for ReactiveSceneDelegate {}
);

impl ReactiveSceneDelegate {
    fn connect(&self, scene: &UIScene) {
        let Some(window_scene) = scene.downcast_ref::<UIWindowScene>() else {
            tracing::warn!("ignoring a scene that is not a UIWindowScene");
            return;
        };

        let Some(setup) = take_setup() else {
            tracing::error!(
                "scene reconnected but the setup function was already used; \
                 start the app with run_app_reconnectable to rebuild the UI"
            );
            return;
        };

        // A delegate serves one scene, but be safe if UIKit reuses it.
        self.disconnect();

        let scope = ReactiveScope::default();
        let mut ctx = SetupContext::new_root(&scope);
        ctx.set_static_context(&WINDOW_SCENE, window_scene.retain());
        setup(&mut ctx);

        let app_loop = AppLoop::new(scope, self.mtm());
        app_loop.schedule_tick();

        self.ivars().replace(Some(SceneState {
            app_loop,
            root: ctx.component_id(),
        }));
    }

    fn disconnect(&self) {
        // Release the borrow before running cleanups, which are user code.
        let state = self.ivars().borrow_mut().take();
        if let Some(state) = state
            && let Some(scope) = state.app_loop.shutdown()
        {
            scope.dispose_component(state.root);
        }
    }
}

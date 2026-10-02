use ui_core::widgets::Platform;

fn main() {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt::init();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let _guard = rt.enter();

    #[cfg(feature = "appkit")]
    run::<ui_core::appkit::platform::AppKit>();

    #[cfg(feature = "gtk")]
    run::<ui_core::gtk::platform::Gtk>();
}

#[allow(dead_code)]
fn run<P: Platform>() {
    P::run_app(flex_demo::setup_demo::<P>);
}

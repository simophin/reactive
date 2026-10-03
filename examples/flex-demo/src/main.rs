fn main() {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt::init();
    flex_demo::run();
}

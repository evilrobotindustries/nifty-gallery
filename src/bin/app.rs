fn main() {
    console_error_panic_hook::set_once();

    wasm_logger::init(wasm_logger::Config::new(log::Level::Trace));
    yew::start_app::<nifty_gallery::App>();
    log::trace!("app started");
}

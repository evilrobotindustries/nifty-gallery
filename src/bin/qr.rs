use workers::PublicWorker;

fn main() {
    console_error_panic_hook::set_once();

    wasm_logger::init(wasm_logger::Config::new(log::Level::Trace));
    log::trace!("starting qr worker...");
    workers::qr::Worker::register();
    log::trace!("qr worker started");
}

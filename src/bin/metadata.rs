use workers::PublicWorker;

fn main() {
    console_error_panic_hook::set_once();

    wasm_logger::init(wasm_logger::Config::new(log::Level::Trace));
    log::trace!("starting metadata worker...");
    workers::metadata::Worker::register();
    log::trace!("metadata worker started");
}

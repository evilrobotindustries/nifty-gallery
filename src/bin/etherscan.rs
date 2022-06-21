use workers::PublicWorker;

fn main() {
    console_error_panic_hook::set_once();

    wasm_logger::init(wasm_logger::Config::new(log::Level::Trace));
    log::trace!("starting etherscan worker...");
    workers::etherscan::Worker::register();
    log::trace!("etherscan worker started");
}

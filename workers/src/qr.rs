use gloo_worker::{HandlerId, Public, WorkerLink};
use qrcode_generator::QrCodeEcc;
use serde::{Deserialize, Serialize};

pub struct Worker {
    link: WorkerLink<Self>,
}

#[derive(Serialize, Deserialize)]
pub struct Request {
    pub url: String,
}

#[derive(Serialize, Deserialize)]
pub struct Response {
    pub qr_code: String,
}

impl gloo_worker::Worker for Worker {
    type Reach = Public<Self>;
    type Message = ();
    type Input = Request;
    type Output = Response;

    fn create(link: WorkerLink<Self>) -> Self {
        log::trace!("creating worker...");
        Self { link }
    }

    fn update(&mut self, _msg: Self::Message) {}

    fn handle_input(&mut self, msg: Self::Input, id: HandlerId) {
        if let Ok(qr_code) = qrcode_generator::to_png_to_vec(&msg.url, QrCodeEcc::Low, 80) {
            log::trace!("qr code generated");
            self.link.respond(
                id,
                Response {
                    qr_code: format!("data:image/png;base64,{}", base64::encode(qr_code)),
                },
            )
        }
    }

    fn name_of_resource() -> &'static str {
        "qr.js"
    }
}

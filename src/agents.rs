use gloo_net::Error;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use yew_agent::{HandlerId, Public, Worker, WorkerLink};

#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
    Metadata { uri: String },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    Completed(crate::metadata::Metadata),
}

pub enum Msg {
    Request(String),
    Completed(crate::metadata::Metadata),
    Redirect(String),
    Failed(String),
    NotFound,
}

pub struct Metadata {
    link: WorkerLink<Self>,
    subscribers: HashSet<HandlerId>,
}

impl Worker for Metadata {
    type Reach = Public<Self>;
    type Message = Msg;
    type Input = Request;
    type Output = Response;

    fn create(link: WorkerLink<Self>) -> Self {
        Self {
            link,
            subscribers: HashSet::new(),
        }
    }

    fn update(&mut self, msg: Self::Message) {
        match msg {
            Msg::Request(uri) => {
                log::trace!("metadata: requesting {uri}...");
                self.link
                    .send_future(async move { request_metadata(uri).await });
            }
            Msg::Completed(metadata) => {
                log::trace!("metadata: completed");
                for id in self.subscribers.iter() {
                    log::trace!("metadata: notifying subscriber");
                    self.link
                        .respond(*id, Response::Completed(metadata.clone()))
                }
            }
            Msg::Redirect(_) => {}
            Msg::Failed(_) => {}
            Msg::NotFound => {}
        }
    }

    fn connected(&mut self, id: HandlerId) {
        self.subscribers.insert(id);
    }

    fn handle_input(&mut self, msg: Self::Input, id: HandlerId) {
        match msg {
            Request::Metadata { uri } => {
                log::trace!("metadata: request received for {uri}");
                self.link.send_message(Msg::Request(uri));
            }
        }
    }

    fn disconnected(&mut self, id: HandlerId) {
        self.subscribers.remove(&id);
    }
}

async fn request_metadata(uri: String) -> Msg {
    log::trace!("metadata: requesting...");
    match gloo_net::http::Request::get(&uri).send().await {
        Ok(response) => match response.status() {
            200 => {
                // Read response as text to handle empty result
                match response.text().await {
                    Ok(response) => {
                        if response.len() == 0 {
                            return Msg::NotFound;
                        }

                        match serde_json::from_str::<crate::metadata::Metadata>(&response) {
                            Ok(metadata) => Msg::Completed(metadata),
                            Err(e) => {
                                log::trace!("{:?}", response);
                                log::error!("{:?}", e);
                                Msg::Failed("An error occurred parsing the metadata".to_string())
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("{:?}", e);
                        Msg::Failed("An error occurred reading the response".to_string())
                    }
                }
            }
            302 => match response.headers().get("location") {
                Some(uri) => Msg::Redirect(uri),
                None => {
                    Msg::Failed("Received 302 Found but location header not present".to_string())
                }
            },
            404 => Msg::NotFound,
            _ => Msg::Failed(format!(
                "Request failed: {} {}",
                response.status(),
                response.status_text()
            )),
        },
        Err(e) => {
            match e {
                Error::JsError(e) => {
                    // Attempt to get status code
                    log::error!("{:?}", e);
                    Msg::Failed(format!("Requesting metadata from {uri} failed: {e}"))
                }
                _ => Msg::Failed(format!("Requesting metadata from {uri} failed: {e}")),
            }
        }
    }
}

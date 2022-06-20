use gloo_net::Error;
use gloo_worker::{HandlerId, Public, WorkerLink};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use url::{ParseError, Url};

/// JSON-specific serialisation/deserialisation, as workers use bincode
mod json;

pub struct Worker {
    link: WorkerLink<Self>,
    subscribers: HashSet<HandlerId>,
}

#[derive(Serialize, Deserialize)]
pub struct Request {
    pub url: String,
}

#[derive(Serialize, Deserialize)]
pub struct Response {
    pub metadata: Metadata,
}

pub enum Msg {
    /// Requests metadata at the specified uri.
    Request(String),
    /// Processes the resulting metadata before completing.
    Process {
        metadata: Metadata,
        /// The (requested) metadata uri
        uri: String,
    },
    Completed(Metadata),
    Redirect(String),
    Failed(String),
    NotFound,
}

impl gloo_worker::Worker for Worker {
    type Reach = Public<Self>;
    type Message = Msg;
    type Input = Request;
    type Output = Response;

    fn create(link: WorkerLink<Self>) -> Self {
        log::trace!("creating worker...");
        Self {
            link,
            subscribers: HashSet::new(),
        }
    }

    fn update(&mut self, msg: Self::Message) {
        log::trace!("updating...");
        match msg {
            Msg::Request(uri) => {
                log::trace!("requesting {uri}...");
                self.link
                    .send_future(async move { request_metadata(uri).await });
            }
            Msg::Process { metadata, uri } => {
                log::trace!("processing");
                // Process the metadata before returning as completed
                let metadata = process(metadata, Url::parse(&uri).expect("could not parse url"));
                self.update(Msg::Completed(metadata));
            }
            Msg::Completed(metadata) => {
                log::trace!("completed..");
                for id in self.subscribers.iter() {
                    log::trace!("notifying subscriber");
                    self.link.respond(
                        *id,
                        Response {
                            metadata: metadata.clone(),
                        },
                    )
                }
            }
            Msg::Redirect(_) => {}
            Msg::Failed(_) => {}
            Msg::NotFound => {}
        }
    }

    fn connected(&mut self, id: HandlerId) {
        log::trace!("connected");
        self.subscribers.insert(id);
    }

    fn handle_input(&mut self, msg: Self::Input, _id: HandlerId) {
        log::trace!("request received for {}", msg.url);
        self.update(Msg::Request(msg.url));
    }

    fn disconnected(&mut self, id: HandlerId) {
        log::trace!("disconnected");
        self.subscribers.remove(&id);
    }

    fn name_of_resource() -> &'static str {
        "metadata.js"
    }
}

fn process(mut metadata: Metadata, url: Url) -> Metadata {
    // Adjust uris
    metadata.image = parse_uri(metadata.image, &url);
    if let Some(uri) = metadata.animation_url {
        metadata.animation_url = Some(parse_uri(uri, &url));
    }
    metadata
}

fn parse_uri(uri: String, base_uri: &Url) -> String {
    if let Err(e) = Url::parse(&uri) {
        // If uri is relative, a
        if matches!(e, ParseError::RelativeUrlWithoutBase) {
            return base_uri.join(&uri).map_or(uri, |url| url.to_string());
        }
    }
    uri
}

async fn request_metadata(uri: String) -> Msg {
    log::trace!("requesting...");
    match crate::fetch::get(&uri).await {
        Ok(response) => match response.status() {
            200 => {
                // Read response as text to handle empty result
                match response.text().await {
                    Ok(response) => {
                        if response.len() == 0 {
                            return Msg::NotFound;
                        }
                        match serde_json::from_str::<json::Metadata>(&response) {
                            Ok(metadata) => Msg::Process {
                                metadata: metadata.into(),
                                uri,
                            },
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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Metadata {
    // Name of the item.
    pub name: Option<String>,
    // A human readable description of the item. Markdown is supported.
    pub description: Option<String>,
    /// This is the URL to the image of the item. Can be just about any type of image (including SVGs, which will be cached into PNGs by OpenSea), and can be IPFS URLs or paths. We recommend using a 350 x 350 image.
    pub image: String,
    // This is the URL that will appear below the asset's image on OpenSea and will allow users to leave OpenSea and view the item on your site.
    pub external_url: Option<String>,
    // These are the attributes for the item, which will show up on the OpenSea page for the item. (see below)
    pub attributes: Vec<Attribute>,
    // Background color of the item on OpenSea. Must be a six-character hexadecimal without a pre-pended #.
    pub background_color: Option<String>,
    //
    pub created_by: Option<String>,
    // A URL to a multi-media attachment for the item. The file extensions GLTF, GLB, WEBM, MP4, M4V, OGV, and OGG are supported, along with the audio-only extensions MP3, WAV, and OGA.
    // Animation_url also supports HTML pages, allowing you to build rich experiences and interactive NFTs using JavaScript canvas, WebGL, and more. Scripts and relative paths within the HTML page are now supported. However, access to browser extensions is not supported.
    pub animation_url: Option<String>,
    // A URL to a YouTube video.
    pub youtube_url: Option<String>,
}

impl From<json::Metadata> for Metadata {
    fn from(metadata: json::Metadata) -> Self {
        Metadata {
            name: metadata.name,
            description: metadata.description,
            image: metadata.image,
            external_url: metadata.external_url,
            attributes: metadata.attributes.into_iter().map(|a| a.into()).collect(),
            background_color: metadata.background_color,
            created_by: metadata.created_by,
            animation_url: metadata.animation_url,
            youtube_url: metadata.youtube_url,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Attribute {
    String {
        trait_type: String,
        value: String,
    },
    // Numeric
    Number {
        trait_type: String,
        value: i64,
        max_value: Option<usize>,
    },
    BoostPercentage {
        trait_type: String,
        value: f64,
        max_value: Option<usize>,
    },
    BoostNumber {
        trait_type: String,
        value: f64,
        max_value: Option<usize>,
    },
    // Date
    Date {
        trait_type: String,
        // A unix timestamp (seconds)
        value: u64,
    },
}

impl Attribute {
    pub fn map(&self) -> (String, String) {
        match self {
            Attribute::String { trait_type, value } => (trait_type.to_string(), value.to_string()),
            Attribute::Number {
                trait_type, value, ..
            } => (trait_type.to_string(), value.to_string()),
            Attribute::BoostPercentage {
                trait_type, value, ..
            } => (trait_type.to_string(), value.to_string()),
            Attribute::BoostNumber {
                trait_type, value, ..
            } => (trait_type.to_string(), value.to_string()),
            Attribute::Date { trait_type, value } => (trait_type.to_string(), value.to_string()),
        }
    }
}

impl From<json::Attribute> for Attribute {
    fn from(attribute: json::Attribute) -> Self {
        match attribute {
            json::Attribute::String { trait_type, value } => {
                Attribute::String { trait_type, value }
            }
            json::Attribute::Number {
                trait_type,
                value,
                max_value,
            } => Attribute::Number {
                trait_type,
                value,
                max_value,
            },
            json::Attribute::BoostPercentage {
                trait_type,
                value,
                max_value,
            } => Attribute::BoostPercentage {
                trait_type,
                value,
                max_value,
            },
            json::Attribute::BoostNumber {
                trait_type,
                value,
                max_value,
            } => Attribute::BoostNumber {
                trait_type,
                value,
                max_value,
            },
            json::Attribute::Date { trait_type, value } => Attribute::Date { trait_type, value },
        }
    }
}

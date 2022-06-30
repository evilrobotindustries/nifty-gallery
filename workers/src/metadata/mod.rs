use async_recursion::async_recursion;
use gloo_net::Error;
use gloo_worker::{HandlerId, Public, WorkerLink};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::Mutex};
use url::{ParseError, Url};

/// JSON-specific serialisation/deserialisation, as workers use bincode
mod json;

pub struct Worker {
    link: WorkerLink<Self>,
}

#[derive(Serialize, Deserialize)]
pub struct Request {
    pub url: String,
    pub token: Option<u32>,
    /// An optional url to be used as a CORS proxy, should the primary request fail
    pub cors_proxy: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub enum Response {
    Completed(String, Option<u32>, Metadata),
    NotFound(String, Option<u32>),
    Failed(String, Option<u32>),
}

pub enum Message {
    /// Requests metadata at the specified uri.
    Request(String, Option<u32>, HandlerId, Option<String>),
    /// Processes the resulting metadata before completing.
    Process {
        metadata: Metadata,
        /// The (requested) metadata uri
        uri: String,
        token: Option<u32>,
        id: HandlerId,
    },
    Completed(String, Option<u32>, Metadata, HandlerId),
    Redirect(String),
    Failed(String, Option<u32>, HandlerId),
    NotFound(String, Option<u32>, HandlerId),
}

impl gloo_worker::Worker for Worker {
    type Reach = Public<Self>;
    type Message = Message;
    type Input = Request;
    type Output = Response;

    fn create(link: WorkerLink<Self>) -> Self {
        log::trace!("creating worker...");
        Self { link }
    }

    fn update(&mut self, msg: Self::Message) {
        log::trace!("updating...");
        match msg {
            Message::Request(uri, token, id, cors_proxy) => {
                log::trace!("requesting {uri}...");
                self.link.send_future(async move {
                    request_metadata(Uri::Standard { uri }, token, id, cors_proxy).await
                });
            }
            Message::Process {
                metadata,
                uri,
                token,
                id,
            } => {
                log::trace!("processing");
                // Process the metadata before returning as completed
                let metadata = process(metadata, Url::parse(&uri).expect("could not parse url"));
                self.update(Message::Completed(uri, token, metadata, id));
            }
            Message::Completed(url, token, metadata, id) => {
                log::trace!("metadata completed");
                self.link
                    .respond(id, Response::Completed(url, token, metadata));
            }
            Message::Redirect(_) => {}
            Message::Failed(url, token, id) => {
                log::trace!("metadata failed at {url}");
                self.link.respond(id, Response::Failed(url, token));
            }
            Message::NotFound(url, token, id) => {
                log::trace!("metadata not found at {url}");
                self.link.respond(id, Response::NotFound(url, token));
            }
        }
    }

    fn handle_input(&mut self, msg: Self::Input, id: HandlerId) {
        log::trace!("request received for {}", msg.url);
        self.update(Message::Request(msg.url, msg.token, id, msg.cors_proxy));
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

static CORS_DOMAINS: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| Mutex::new(HashSet::new()));

#[async_recursion(?Send)]
async fn request_metadata(
    mut request: Uri,
    token: Option<u32>,
    id: HandlerId,
    cors_proxy: Option<String>,
) -> Message {
    log::trace!("requesting...");

    // Check if standard uri should use cors proxy (based on previous requests for same host)
    if let Uri::Standard { uri } = &request {
        if let Some(ref host) = request.host() {
            if CORS_DOMAINS.lock().unwrap().contains(host) {
                if let Some(proxy) = &cors_proxy {
                    // Update request to use proxy, appending original uri to proxy address as parameter
                    log::trace!("using cors proxy...");
                    request = Uri::proxy(uri, proxy)
                }
            }
        }
    }

    match crate::fetch::get(&request.effective_uri()).await {
        Ok(response) => match response.status() {
            200 => {
                // Read response as text to handle empty result
                match response.text().await {
                    Ok(response) => {
                        if response.len() == 0 {
                            return Message::NotFound(
                                request.original_uri().to_string(),
                                token,
                                id,
                            );
                        }
                        match serde_json::from_str::<json::Metadata>(&response) {
                            Ok(metadata) => Message::Process {
                                metadata: metadata.into(),
                                uri: request.original_uri().to_string(),
                                token,
                                id,
                            },
                            Err(e) => {
                                log::trace!("{:?}", response);
                                log::error!("{:?}", e);
                                Message::Failed(
                                    "An error occurred parsing the metadata".to_string(),
                                    token,
                                    id,
                                )
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("{:?}", e);
                        Message::Failed(
                            "An error occurred reading the response".to_string(),
                            token,
                            id,
                        )
                    }
                }
            }
            302 => match response.headers().get("location") {
                Some(uri) => Message::Redirect(uri),
                None => Message::Failed(
                    "Received 302 Found but location header not present".to_string(),
                    token,
                    id,
                ),
            },
            404 => Message::NotFound(request.original_uri().to_string(), token, id),
            _ => Message::Failed(
                format!(
                    "Request failed: {} {}",
                    response.status(),
                    response.status_text()
                ),
                token,
                id,
            ),
        },
        Err(e) => {
            match e {
                Error::JsError(e) => {
                    // Assume JS error is CORS related and re-attempt standard request via CORS proxy (if specified)
                    if let Uri::Standard { uri } = &request {
                        if let Some(proxy) = &cors_proxy {
                            log::info!("request failed, re-attempting via cors proxy...");
                            let proxied_result =
                                request_metadata(Uri::proxy(uri, proxy), token, id, None).await;
                            if !matches!(proxied_result, Message::Failed(_, _, _)) {
                                if let Some(host) = request.host() {
                                    log::trace!("cors proxy successful, adding host to cors list for future requests");
                                    CORS_DOMAINS.lock().unwrap().insert(host);
                                }
                            }

                            return proxied_result;
                        }
                    }

                    // Attempt to get status code
                    log::error!("{:?}", e);
                    Message::Failed(
                        format!(
                            "Requesting metadata from {} failed: {e}",
                            &request.original_uri()
                        ),
                        token,
                        id,
                    )
                }
                _ => Message::Failed(
                    format!(
                        "Requesting metadata from {} failed: {e}",
                        &request.original_uri()
                    ),
                    token,
                    id,
                ),
            }
        }
    }
}

enum Uri {
    Standard { uri: String },
    Proxied { uri: String, original: String },
}

impl Uri {
    fn host(&self) -> Option<String> {
        Url::parse(self.original_uri())
            .ok()
            .and_then(|url| url.host_str().map(|host| host.to_string()))
    }

    fn original_uri(&self) -> &str {
        match self {
            Uri::Standard { uri } => uri,
            Uri::Proxied { original, .. } => original,
        }
    }

    fn effective_uri(&self) -> &str {
        match self {
            Uri::Standard { uri } => uri,
            Uri::Proxied { uri, .. } => uri,
        }
    }

    fn proxy(uri: &str, proxy: &str) -> Uri {
        Uri::Proxied {
            uri: format!("{proxy}{uri}"),
            original: uri.to_string(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Metadata {
    // Name of the item.
    #[serde(rename = "n")]
    pub name: Option<String>,
    // A human readable description of the item. Markdown is supported.
    #[serde(rename = "d")]
    pub description: Option<String>,
    /// This is the URL to the image of the item. Can be just about any type of image (including SVGs, which will be cached into PNGs by OpenSea), and can be IPFS URLs or paths. We recommend using a 350 x 350 image.
    #[serde(rename = "i")]
    pub image: String,
    // This is the URL that will appear below the asset's image on OpenSea and will allow users to leave OpenSea and view the item on your site.
    #[serde(rename = "eu")]
    pub external_url: Option<String>,
    // These are the attributes for the item, which will show up on the OpenSea page for the item. (see below)
    #[serde(rename = "a")]
    pub attributes: Vec<Attribute>,
    // Background color of the item on OpenSea. Must be a six-character hexadecimal without a pre-pended #.
    #[serde(rename = "bc")]
    pub background_color: Option<String>,
    //
    #[serde(rename = "cb")]
    pub created_by: Option<String>,
    // A URL to a multi-media attachment for the item. The file extensions GLTF, GLB, WEBM, MP4, M4V, OGV, and OGG are supported, along with the audio-only extensions MP3, WAV, and OGA.
    // Animation_url also supports HTML pages, allowing you to build rich experiences and interactive NFTs using JavaScript canvas, WebGL, and more. Scripts and relative paths within the HTML page are now supported. However, access to browser extensions is not supported.
    #[serde(rename = "au")]
    pub animation_url: Option<String>,
    // A URL to a YouTube video.
    #[serde(rename = "yu")]
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
        #[serde(rename = "tt")]
        trait_type: String,
        #[serde(rename = "v")]
        value: String,
    },
    // Numeric
    Number {
        #[serde(rename = "tt")]
        trait_type: String,
        #[serde(rename = "v")]
        value: i64,
        #[serde(rename = "mv")]
        max_value: Option<usize>,
    },
    BoostPercentage {
        #[serde(rename = "tt")]
        trait_type: String,
        #[serde(rename = "v")]
        value: f64,
        #[serde(rename = "mv")]
        max_value: Option<usize>,
    },
    BoostNumber {
        #[serde(rename = "tt")]
        trait_type: String,
        #[serde(rename = "v")]
        value: f64,
        #[serde(rename = "mv")]
        max_value: Option<usize>,
    },
    // Date
    Date {
        #[serde(rename = "tt")]
        trait_type: String,
        // A unix timestamp (seconds)
        #[serde(rename = "v")]
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

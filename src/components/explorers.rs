use crate::uri::Uri;
use crate::Route;
use gloo_console::{debug, error};
use gloo_net::http::Request;
use gloo_net::Error;
use itertools::Itertools;
use qrcode_generator::QrCodeEcc;
use std::collections::HashMap;
use std::str;
use url::{ParseError, Url};
use web_sys::Document;
use yew::prelude::*;
use yew_router::prelude::*;

#[function_component(Address)]
pub fn address() -> yew::Html {
    html! {}
}

pub enum Msg {
    RequestMetadata,
    MetadataLoaded(crate::metadata::Metadata),
    RequestFailed(String),
    NotFound,
    Navigate(usize),
}

#[derive(PartialEq, Properties)]
pub struct Props {
    pub id: String,
    pub token: usize,
}

pub struct Collection {
    base_uri: Option<String>,
    start_token: usize,
    current_token: usize,
    error: Option<String>,
    requesting_metadata: bool,
    metadata: Option<MetadataProps>,
    document: Document,
}

impl Component for Collection {
    type Message = Msg;
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        if let Err(e) = yew_router_qs::try_route_from_query_string() {
            error!(e)
        }

        _ctx.link().send_message(Msg::RequestMetadata);

        let window = web_sys::window().expect("global window does not exists");
        let document = window.document().expect("expecting a document on window");

        Self {
            base_uri: Some(Uri::decode(&_ctx.props().id).unwrap().to_string()),
            start_token: 0,
            current_token: _ctx.props().token,
            error: None,
            requesting_metadata: false,
            metadata: None,
            document,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::RequestMetadata => {
                // Set uri for current token and request
                let uri = format!("{}{}", self.base_uri.as_ref().unwrap(), self.current_token);
                ctx.link().send_future(async move {
                    match Request::get(&uri).send().await {
                        Ok(response) => match response.status() {
                            200 => {
                                // Read response as text to handle empty result
                                match response.text().await {
                                    Ok(response) => {
                                        if response.len() == 0 {
                                            return Msg::NotFound;
                                        }

                                        match serde_json::from_str::<crate::metadata::Metadata>(
                                            &response,
                                        ) {
                                            Ok(metadata) => Msg::MetadataLoaded(metadata),
                                            Err(e) => {
                                                error!(format!("{:?}", e));
                                                Msg::RequestFailed(
                                                    "An error occurred parsing the metadata"
                                                        .to_string(),
                                                )
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        error!(format!("{:?}", e));
                                        Msg::RequestFailed(
                                            "An error occurred reading the response".to_string(),
                                        )
                                    }
                                }
                            }
                            404 => Msg::NotFound,
                            _ => Msg::RequestFailed(format!(
                                "Request failed: {} {}",
                                response.status(),
                                response.status_text()
                            )),
                        },
                        Err(e) => {
                            match e {
                                Error::JsError(e) => {
                                    // Attempt to get status code
                                    error!(format!("{:?}", e));
                                    Msg::RequestFailed(format!(
                                        "Requesting metadata from {uri} failed: {e}"
                                    ))
                                }
                                _ => Msg::RequestFailed(format!(
                                    "Requesting metadata from {uri} failed: {e}"
                                )),
                            }
                        }
                    }
                });
                self.error = None;
                self.requesting_metadata = true;
                true
            }
            Msg::MetadataLoaded(metadata) => {
                self.requesting_metadata = false;
                self.metadata = Some(MetadataProps::from(
                    self.current_token,
                    &metadata,
                    self.base_uri.as_ref(),
                    &self.document,
                ));
                true
            }
            Msg::RequestFailed(error) => {
                self.requesting_metadata = false;
                self.error = Some(error);
                true
            }
            Msg::NotFound => {
                // If current token zero and not found, collection might start at one so advance
                if self.current_token == 0 {
                    self.start_token = 1;
                    ctx.link().send_message(Msg::Navigate(self.start_token));
                    return false;
                }

                self.requesting_metadata = false;
                self.metadata = None;
                true
            }
            Msg::Navigate(token) => {
                self.current_token = token;
                ctx.link().send_message(Msg::RequestMetadata);
                ctx.link()
                    .history()
                    .expect("")
                    .push(Route::CollectionToken {
                        id: Uri::encode(self.base_uri.as_ref().unwrap()),
                        token: self.current_token,
                    });
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let current_token = self.current_token;
        let previous_click = ctx
            .link()
            .callback(move |_| Msg::Navigate(current_token - 1));
        let next_click = ctx
            .link()
            .callback(move |_| Msg::Navigate(current_token + 1));
        let start_token = self.start_token;
        let return_to_start_click = ctx.link().callback(move |_| Msg::Navigate(start_token));

        html! {
                <section class="section is-fullheight">
                    if let Some(error) = &self.error {
                        <div class="notification is-danger">
                          { error }
                        </div>
                    }

                    if let Some(metadata) = self.metadata.as_ref() {
                        <div class="card columns">
                            <div class="column">
                                <figure class="image">
                                    <img src={ metadata.image.clone() } alt={ metadata.name.clone() } class="modal-button"
                                         data-target="nifty-image" />
                                </figure>
                                <div id="nifty-image" class="modal modal-fx-3dFlipHorizontal">
                                    <div class="modal-background"></div>
                                    <div class="modal-content">
                                        <p class="image">
                                            <img src={ metadata.image.clone() } alt={ metadata.name.clone() } />
                                        </p>
                                    </div>
                                    <button class="modal-close is-large" aria-label="close"></button>
                                </div>
                            </div>
                            <div class="column">
                                <div class="card-content">
                                    <div class="level is-mobile">
                                        <div class="level-left">
                                            <div class="level-item">
                                                <h1 class="title">{ &metadata.name }</h1>
                                            </div>
                                        </div>
                                        <div class="level-right">
                                            <div class="field has-addons">
                                              <div class="control">
                                                <button class="button is-primary" onclick={ previous_click }
                                                        disabled={ self.requesting_metadata || self.current_token == self.start_token }>
                                                    <span class="icon is-small">
                                                      <i class="fas fa-angle-left"></i>
                                                    </span>
                                                </button>
                                              </div>
                                              <div class="control">
                                                <button class="button is-primary" onclick={ next_click }
                                                        disabled={ self.requesting_metadata }>
                                                    <span class="icon is-small">
                                                      <i class="fas fa-angle-right"></i>
                                                    </span>
                                                </button>
                                              </div>
                                            </div>
                                        </div>
                                    </div>
                                    <div class="content">{ &metadata.description }</div>
                                    <div class="field is-grouped is-grouped-multiline">{ metadata.attributes() }</div>
                                </div>
                                <footer class="card-footer">
                                    <div class="card-content level is-mobile">
                                        <div class="level-left">
                                            <div class="level-item has-text-centered">
                                                <div>
                                                    <p class="heading">{"Traits"}</p>
                                                    <p class="title">{ metadata.traits() }</p>
                                                </div>
                                            </div>
                                        </div>
                                        <div class="level-right">
                                            if let Some(qr_code) = metadata.qr_code() {
                                                <figure class="image is-qr-code level-item">
                                                    <img src={ qr_code } alt={ metadata.name.clone() } />
                                                </figure>
                                            }
                                        </div>
                                    </div>
                                </footer>
                            </div>
                        </div>
                    }
                    else {
                        if !self.requesting_metadata && self.current_token != self.start_token {
                            <article class="message is-primary">
                                <div class="message-body">
                                    {"The requested token was not found. Have you reached the end of the collection? Click "}
                                    <a href="javascript:void(0);" onclick={return_to_start_click}>{"here"}</a>
                                    {" to return to the start of the collection."}
                                </div>
                            </article>
                        }
                    }
                </section>
        }
    }

    fn rendered(&mut self, _ctx: &Context<Self>, _first_render: bool) {
        // Wire up full screen image modal
        bulma::add_modals(&self.document);
    }
}

fn parse_uri(uri: &str) -> Result<Url, ParseError> {
    let mut url = Url::parse(&uri)?;
    if url.scheme() == "ipfs" {
        // Convert IPFS protocol address to IPFS gateway
        // ( preserve existing object to preserve additional attributes like query string parameters etc.)
        let cid = url
            .host_str()
            .expect("could not get host name from url")
            .to_string();
        url.set_host(Some("ipfs.io"))?;
        url.set_path(&format!("/ipfs/{}{}", cid, url.path()));

        // New instance required due to internal url rules about changing schemes
        url = Url::parse(&url.to_string().replace("ipfs://", "https://"))
            .expect("could not parse url converted from ipfs to https")
    }
    Ok(url)
}

#[derive(Properties, PartialEq, Clone)]
pub struct MetadataProps {
    pub id: usize,
    pub name: String,
    pub description: String,
    pub image: String,
    pub attributes: HashMap<String, String>,
    pub external_url: Option<String>,
    pub qr_code: Option<String>,
}

impl MetadataProps {
    fn from(
        id: usize,
        metadata: &crate::metadata::Metadata,
        base_uri: Option<&String>,
        document: &Document,
    ) -> MetadataProps {
        let image = if metadata.image.starts_with(".") {
            parse_uri(base_uri.unwrap())
                .unwrap()
                .join(&metadata.image)
                .unwrap()
                .to_string()
        } else {
            parse_uri(&metadata.image).unwrap().to_string()
        };

        let location = document
            .location()
            .expect("could not get document location")
            .href()
            .expect("could not get document location href as string");
        let qr_code = match qrcode_generator::to_png_to_vec(location, QrCodeEcc::Low, 128) {
            Ok(qr_code) => Some(base64::encode(qr_code)),
            Err(_) => None,
        };

        MetadataProps {
            id,
            name: metadata.name.clone(),
            description: metadata.description.clone(),
            image,
            attributes: metadata.attributes.iter().map(|a| a.map()).collect(),
            external_url: None,
            qr_code,
        }
    }

    fn attributes(&self) -> Html {
        self.attributes
            .iter()
            .sorted_by_key(|a| a.0)
            .map(|a| {
                html! {
                    <div class="control">
                        <div class="tags has-addons">
                            <span class="tag">{a.0}</span>
                            <span class="tag">{a.1}</span>
                        </div>
                    </div>
                }
            })
            .collect()
    }

    fn qr_code(&self) -> Option<String> {
        self.qr_code
            .as_ref()
            .map(|base64| format!("data:image/png;base64,{base64}"))
    }

    fn traits(&self) -> usize {
        self.attributes.iter().filter(|a| a.1 != "None").count()
    }
}

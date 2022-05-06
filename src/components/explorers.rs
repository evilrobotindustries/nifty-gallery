use gloo_console::{debug, error};
use gloo_net::http::Request;
use gloo_net::Error;
use itertools::Itertools;
use qrcode_generator::QrCodeEcc;
use std::collections::HashMap;
use std::str::FromStr;
use url::{ParseError, Url};
use web_sys::HtmlInputElement;
use yew::prelude::*;

#[function_component(Address)]
pub fn address() -> yew::Html {
    html! {}
}

pub enum Msg {
    UriChanged(String),
    UriFailed(String),
    RequestMetadata,
    MetadataLoaded(crate::metadata::Metadata),
    NotFound,
    // Navigation
    Previous,
    Next,
    ReturnToStart,
}

pub struct Collection {
    base_uri: Option<String>,
    start_token: usize,
    current_token: usize,
    error: Option<String>,
    requesting_metadata: bool,
    metadata: Option<crate::metadata::Metadata>,
}

impl Component for Collection {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        if let Err(e) = yew_router_qs::try_route_from_query_string() {
            error!(e)
        }

        Self {
            base_uri: None,
            start_token: 0,
            current_token: 0,
            error: None,
            requesting_metadata: false,
            metadata: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::UriChanged(uri) => {
                self.error = None;

                if uri == "" {
                    return false;
                }

                // parse uri
                match parse_uri(&uri) {
                    Ok(url) => {
                        // Get token from path
                        let segments: Vec<&str> = url
                            .path_segments()
                            .expect("could not get path segments from url")
                            .collect();

                        match segments.last() {
                            Some(token) => {
                                // Set base uri and token
                                let uri = url.to_string();
                                self.base_uri = Some(uri[..uri.len() - token.len()].to_string());
                                match usize::from_str(token) {
                                    Ok(token) => {
                                        self.current_token = token;
                                        ctx.link().send_message(Msg::RequestMetadata);
                                        false
                                    }
                                    Err(e) => {
                                        self.error =
                                            Some(format!("could not parse the token id: {:?}", e));
                                        true
                                    }
                                }
                            }
                            None => {
                                self.error = Some(format!("could not parse the token id"));
                                true
                            }
                        }
                    }
                    Err(e) => {
                        self.error = Some("Could not parse the URL".to_string());
                        error!(format!("could not parse the url: {:?}", e));
                        true
                    }
                }
            }
            Msg::UriFailed(error) => {
                self.requesting_metadata = false;
                self.error = Some(error);
                true
            }
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
                                                Msg::UriFailed(
                                                    "An error occurred parsing the metadata"
                                                        .to_string(),
                                                )
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        error!(format!("{:?}", e));
                                        Msg::UriFailed(
                                            "An error occurred reading the response".to_string(),
                                        )
                                    }
                                }
                            }
                            404 => Msg::NotFound,
                            _ => Msg::UriFailed(format!(
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
                                    Msg::UriFailed(format!(
                                        "Requesting metadata from {uri} failed: {e}"
                                    ))
                                }
                                _ => Msg::UriFailed(format!(
                                    "Requesting metadata from {uri} failed: {e}"
                                )),
                            }
                        }
                    }
                });
                self.metadata = None;
                self.error = None;
                self.requesting_metadata = true;
                true
            }
            Msg::MetadataLoaded(metadata) => {
                self.requesting_metadata = false;
                self.metadata = Some(metadata);
                true
            }
            Msg::NotFound => {
                // If current token zero and not found, collection might start at one so advance
                if self.current_token == 0 {
                    self.start_token = 1;
                    ctx.link().send_message(Msg::Next);
                    return false;
                }

                self.requesting_metadata = false;
                self.metadata = None;
                true
            }
            // Navigation
            Msg::Previous => {
                self.current_token -= 1;
                ctx.link().send_message(Msg::RequestMetadata);
                false
            }
            Msg::Next => {
                self.current_token += 1;
                ctx.link().send_message(Msg::RequestMetadata);
                false
            }
            Msg::ReturnToStart => {
                self.current_token = self.start_token;
                ctx.link().send_message(Msg::RequestMetadata);
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let uri = self
            .base_uri
            .as_ref()
            .map_or("".to_string(), |u| format!("{u}{}", self.current_token));
        let uri_change = ctx.link().callback(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            Msg::UriChanged(input.value())
        });
        let previous_click = ctx.link().callback(|_| Msg::Previous);
        let next_click = ctx.link().callback(|_| Msg::Next);
        let return_to_start_click = ctx.link().callback(|_| Msg::ReturnToStart);
        let error = self
            .error
            .as_ref()
            .map_or("".to_string(), |u| u.to_string());
        html! {
                <section class="section is-fullheight">
                    <div class="form">
                        <div class="field is-horizontal">
                            <div class="field-label is-normal">
                                <label class="label">{"URL"}</label>
                            </div>
                            <div class="field-body">
                                <div class="field">
                                    <div class={classes!("control", "has-icons-left",
                                        self.requesting_metadata.then(|| Some("is-loading")))}>
                                        <input class="input" type="text" placeholder="Enter token URL" value={ uri }
                                               onchange={ uri_change } disabled={ self.requesting_metadata } />
                                        <span class="icon is-small is-left">
                                            <i class="fas fa-globe"></i>
                                        </span>
                                    </div>
                                </div>
                                if self.metadata.is_some() {
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
                                                disabled={ self.requesting_metadata } >
                                            <span class="icon is-small">
                                              <i class="fas fa-angle-right"></i>
                                            </span>
                                        </button>
                                      </div>
                                    </div>
                                }
                            </div>
                        </div>
                    </div>

                    if self.error.is_some() {
                        <div class="notification is-danger">
                          { error }
                        </div>
                    }

                    if self.metadata.is_some() {
                        <Metadata
                            ..MetadataProps::from(self.metadata.as_ref().unwrap(), self.base_uri.as_ref()) />
                    }
                    else {
                        if !self.requesting_metadata && self.current_token != self.start_token {
                            <article class="message is-primary">
                                <div class="message-body">
                                    {"The requested token was not found. Have you reached the end of the collection? Click "}
                                    <a onclick={return_to_start_click}>{"here"}</a>{" to return to the start of the collection."}
                                </div>
                            </article>
                        }
                    }
                </section>
        }
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

#[derive(Properties, PartialEq)]
pub struct MetadataProps {
    pub name: String,
    pub description: String,
    pub image: String,
    pub attributes: HashMap<String, String>,
    pub external_url: Option<String>,
    pub qr_code: Option<String>,
}

impl MetadataProps {
    fn from(metadata: &crate::metadata::Metadata, base_uri: Option<&String>) -> MetadataProps {
        let image = if metadata.image.starts_with(".") {
            parse_uri(base_uri.unwrap())
                .unwrap()
                .join(&metadata.image)
                .unwrap()
                .to_string()
        } else {
            parse_uri(&metadata.image).unwrap().to_string()
        };

        let qr_code =
            match qrcode_generator::to_png_to_vec(metadata.name.clone(), QrCodeEcc::Low, 128) {
                Ok(qr_code) => Some(base64::encode(qr_code)),
                Err(_) => None,
            };

        MetadataProps {
            name: metadata.name.clone(),
            description: metadata.description.clone(),
            image,
            attributes: metadata.attributes.iter().map(|a| a.map()).collect(),
            external_url: None,
            qr_code,
        }
    }
}

#[function_component(Metadata)]
fn metadata(props: &MetadataProps) -> yew::Html {
    use_effect(move || {
        // Wire up full screen image modal
        let window = web_sys::window().expect("global window does not exists");
        let document = window.document().expect("expecting a document on window");
        bulma::add_modals(&document);
        || ()
    });

    let attributes: Html = props
        .attributes
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
        .collect();

    let traits = props.attributes.iter().filter(|a| a.1 != "None").count();

    let qr_code = props
        .qr_code
        .as_ref()
        .map(|base64| format!("data:image/png;base64,{base64}"));

    html! {
        // todo: better layout
        // todo: click image for full screen modal
        // todo: add qr code
        <div class="card columns">
            <div class="column">
                <figure class="image">
                    <img src={ props.image.clone() } alt={ props.name.clone() } class="modal-button" data-target="nifty-image" />
                </figure>
                <div id="nifty-image" class="modal modal-fx-3dFlipHorizontal">
                    <div class="modal-background"></div>
                    <div class="modal-content">
                        <p class="image">
                            <img src={ props.image.clone() } alt={ props.name.clone() } />
                        </p>
                    </div>
                    <button class="modal-close is-large" aria-label="close"></button>
                </div>
            </div>
            <div class="column">
                <div class="card-content">
                    <h1 class="title">{ &props.name }</h1>
                    <div class="content">{ &props.description }</div>
                    <div class="field is-grouped is-grouped-multiline">{ attributes }</div>
                </div>
                <footer class="card-footer">
                    <div class="card-content level is-mobile">
                        <div class="level-left">
                            <div class="level-item has-text-centered">
                                <div>
                                    <p class="heading">{"Traits"}</p>
                                    <p class="title">{ traits }</p>
                                </div>
                            </div>
                        </div>
                        <div class="level-right">
                            if qr_code.is_some() {
                                <figure class="image is-qr-code level-item">
                                    <img src={ qr_code.unwrap() } />
                                </figure>
                            }
                        </div>
                    </div>
                </footer>
            </div>
        </div>
    }
}

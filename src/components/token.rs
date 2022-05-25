use crate::components::token;
use crate::uri::Uri;
use crate::Route;
use bulma::carousel::Options;
use chrono::DateTime;
use gloo_console::{debug, error};
use gloo_net::http::Request;
use gloo_net::Error;
use gloo_storage::errors::StorageError;
use gloo_storage::{LocalStorage, Storage};
use itertools::{rev, Itertools};
use qrcode_generator::QrCodeEcc;
use url::ParseError;
use std::collections::HashMap;
use yew::prelude::*;
use yew_router::prelude::*;

const TOKENS: &str = "Tokens:Viewed";
const CACHE_SIZE: usize = 10;

#[derive(Debug)]
pub enum Msg {
    Request(crate::models::Token),
    Redirect(String),
    Completed(crate::models::Token),
    Failed(String),
    NotFound,
}

#[derive(PartialEq, Properties)]
pub struct Props {
    // The (encoded) token uri, used to identify a collection and to fetch token metadata
    pub token_uri: String,
    // If applicable, the token identifier
    pub token_id: Option<usize>,
    #[prop_or_default]
    pub status: Callback<Status>,
}

pub enum Status {
    NotStarted,
    Requesting,
    Completed,
    NotFound,
    Failed,
}

pub struct Token {
    // The current token
    token: Option<crate::models::Token>,
    // Any error, if applicable
    error: Option<String>,
}

impl Token {
    fn attributes(&self) -> Html {
        self.token.as_ref().map_or(Html::default(), |token| {
            token.metadata.as_ref().map_or(Html::default(), |metadata| {
                let attributes: Vec<(String, String)> =
                    metadata.attributes.iter().map(|a| a.map()).collect();

                attributes
                    .iter()
                    .sorted_by_key(|a| &a.0)
                    .map(|a| {
                        html! {
                            <div class="control">
                                <div class="tags has-addons">
                                    <span class="tag">{ &a.0 }</span>
                                    <span class="tag">{ &a.1 }</span>
                                </div>
                            </div>
                        }
                    })
                    .collect()
            })
        })
    }

    fn description(&self) -> &str {
        self.token.as_ref().map_or("", |token| {
            token.metadata.as_ref().map_or("", |metadata| {
                metadata
                    .description
                    .as_ref()
                    .map_or("", |description| description)
            })
        })
    }

    fn image(&self, ctx: &Context<Token>) -> Option<String> {
        self.token.as_ref().map_or(None, |token| {
            token.metadata.as_ref().map_or(None, |metadata| {
                match Uri::parse(&metadata.image, false) {
                    Ok(uri) => Some(uri.to_string().into()),
                    Err(e) => match e {
                        ParseError::RelativeUrlWithoutBase => {
                            match Uri::parse(&ctx.props().uri, false) {
                                Ok(uri) => Some(uri.join(&metadata.image).to_string().into()),
                                Err(e) => {
                                    error!(format!("{:?}", e));
                                    None
                                }
                            }
                        }
                        _ => {
                            error!(format!("{:?}", e));
                            None
                        }
                    },
                }
            })
        })
    }

    fn name(&self, ctx: &Context<Token>) -> String {
        self.token.as_ref().map_or("".to_string(), |token| {
            token.metadata.as_ref().map_or("".to_string(), |metadata| {
                metadata.name.as_ref().map_or(
                    // Use token number for missing name (if available)
                    token.id.map_or("".to_string(), |token| token.to_string()),
                    |name| name.to_string(),
                )
            })
        })
    }

    fn qr_code(&self) -> Option<String> {
        let window = web_sys::window().expect("global window does not exists");
        let document = window.document().expect("expecting a document on window");
        let location = document
            .location()
            .expect("could not get document location")
            .href()
            .expect("could not get document location href as string");

        match qrcode_generator::to_png_to_vec(&location, QrCodeEcc::Low, 128) {
            Ok(qr_code) => Some(format!("data:image/png;base64,{}", base64::encode(qr_code))),
            Err(_) => None,
        }
    }

    fn traits(&self) -> usize {
        self.token.as_ref().map_or(0, |token| {
            token.metadata.as_ref().map_or(0, |metadata| {
                metadata
                    .attributes
                    .iter()
                    .map(|a| a.map())
                    .filter(|a| a.1 != "None")
                    .count()
            })
        })
    }

    fn video(&self, ctx: &Context<Token>) -> Option<(String, String)> {
        let poster = self.image(ctx).unwrap_or("".to_string());
        match &self.metadata {
            None => None,
            Some(metadata) => match &metadata.animation_url {
                None => None,
                Some(animation_url) => match Uri::parse(animation_url, false) {
                    Ok(uri) => Some((uri.to_string().into(), poster)),
                    Err(e) => match e {
                        ParseError::RelativeUrlWithoutBase => {
                            match Uri::parse(&ctx.props().uri, false) {
                                Ok(uri) => {
                                    Some((uri.join(&animation_url).to_string().into(), poster))
                                }
                                Err(e) => {
                                    error!(format!("{:?}", e));
                                    None
                                }
                            }
                        }
                        _ => {
                            error!(format!("{:?}", e));
                            None
                        }
                    },
                },
            },
        }
    }
}

impl Component for Token {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        let token = crate::models::Token::create(
            Uri::decode(&ctx.props().token_uri).expect("unable to decode the uri"),
            ctx.props().token_id,
        );
        ctx.link().send_message(Msg::Request(token));
        Self {
            token: None,
            error: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Request(token) => {
                self.error = None;

                // Check cache
                let uri = token.url();
                match cache() {
                    Ok(cache) => {
                        if let Some(token) = cache.get(&uri) {
                            ctx.link().send_message(Msg::Completed(token.clone()));
                            return true;
                        }
                    }
                    Err(e) => {
                        if !matches!(e, StorageError::KeyNotFound(_)) {
                            clear_cache();
                            error!(format!("{:?}", e))
                        }
                    }
                }

                ctx.link()
                    .send_future(async move { token::request_metadata(token).await });

                ctx.props().status.emit(Status::Requesting);
                true
            }
            Msg::Redirect(uri) => {
                todo!();
                // ctx.link()
                //     .send_future(async move { token::request_metadata(&uri).await });
                // self.error = None;
                // ctx.props().status.emit(Status::Requesting);
                // true
            }
            Msg::Completed(mut token) => {
                // Update token
                ctx.props().status.emit(Status::Completed);
                self.token = Some(token.clone());

                // Cache token
                token.last_viewed = Some(chrono::offset::Utc::now());
                let mut cache = cache().unwrap_or(HashMap::new());
                if cache.len() >= CACHE_SIZE {
                    let expired: Vec<String> = cache
                        .iter()
                        .sorted_by_key(|(key, value)| {
                            value.last_viewed.unwrap_or(chrono::offset::Utc::now())
                        })
                        .take(cache.len() - CACHE_SIZE + 1)
                        .map(|(key, _)| key.clone())
                        .collect();
                    for key in expired {
                        cache.remove(&key);
                    }
                }
                cache.insert(token.url(), token);
                if let Err(e) = LocalStorage::set(TOKENS, cache) {
                    error!(format!(
                        "An error occurred whilst caching the token: {:?}",
                        e
                    ))
                }

                true
            }
            Msg::Failed(error) => {
                ctx.props().status.emit(Status::Failed);
                self.error = Some(error);
                true
            }
            Msg::NotFound => {
                ctx.props().status.emit(Status::NotFound);
                true
            }
        }
    }

    fn changed(&mut self, ctx: &Context<Self>) -> bool {
        let uri = Uri::decode(&ctx.props().token_uri).expect("unable to decode the uri");
        let id = ctx.props().token_id;
        if self.token.is_none()
            || uri != self.token.as_ref().unwrap().uri
            || id != self.token.as_ref().unwrap().id
        {
            ctx.link()
                .send_message(Msg::Request(crate::models::Token::create(uri, id)));
        }
        false
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <>
                if let Some(error) = &self.error {
                    <div class="notification is-danger">
                      { error }
                    </div>
                }

                if let Some(token) = self.token.as_ref() {
                    if let Some(metadata) = token.metadata.as_ref() {
                        <div class="card columns">
                            if let Some((video, poster)) = self.video(ctx) {
                            <div class="column">
                                <figure class="image">
                                    <video class="modal-button" data-target="nifty-image" controls={true}
                                            poster={ poster.clone() }>
                                        <source src={ video.clone() } type="video/mp4" />
                                    </video>
                                </figure>
                                <div id="nifty-image" class="modal modal-fx-3dFlipHorizontal">
                                    <div class="modal-background"></div>
                                    <div class="modal-content">
                                        <p class="image">
                                            <video class="modal-button" data-target="nifty-image" controls={true}
                                                    poster={ poster }>
                                                <source src={ video } type="video/mp4" />
                                            </video>
                                        </p>
                                    </div>
                                    <button class="modal-close is-large" aria-label="close"></button>
                                </div>
                            </div>
                        }
                        else if let Some(image) = self.image(ctx) {
                                <div class="column">
                                    <figure class="image">
                                        <img src={ image.clone() } alt={ metadata.name.clone() } class="modal-button"
                                             data-target="nifty-image" />
                                    </figure>
                                    <div id="nifty-image" class="modal modal-fx-3dFlipHorizontal">
                                        <div class="modal-background"></div>
                                        <div class="modal-content">
                                            <p class="image">
                                                <img src={ image } alt={ metadata.name.clone() } />
                                            </p>
                                        </div>
                                        <button class="modal-close is-large" aria-label="close"></button>
                                    </div>
                                </div>
                            }
                            <div class="column">
                                <div class="card-content">
                                    <h1 class="title nifty-name">{ self.name(ctx) }</h1>
                                    <div class="content">{ self.description() }</div>
                                    <div class="field is-grouped is-grouped-multiline">{ self.attributes() }</div>
                                    if let Some(external_url) = &metadata.external_url {
                                        <div class="content">
                                            <a href={ external_url.to_string() } target="_blank">
                                                <i class="fa-solid fa-globe"></i>
                                            </a>
                                        </div>
                                    }
                                    <table class="table">
                                    <tbody>
                                    if let Some(last_viewed) = &token.last_viewed {
                                        <tr>
                                            <th>{"Last viewed: "}</th>
                                            <td>{ last_viewed }</td>
                                        </tr>
                                    }
                                    </tbody>
                                    </table>
                                </div>
                                <footer class="card-footer">
                                    <div class="card-content level is-mobile">
                                        <div class="level-left">
                                            <div class="level-item has-text-centered">
                                                <div>
                                                    <p class="heading">{"Traits"}</p>
                                                    <p class="title">{ self.traits() }</p>
                                                </div>
                                            </div>
                                        </div>
                                        <div class="level-right">
                                            if let Some(qr_code) = self.qr_code() {
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
                }
            </>
        }
    }

    fn rendered(&mut self, _ctx: &Context<Self>, _first_render: bool) {
        // Wire up full screen image modal
        let window = web_sys::window().expect("global window does not exists");
        let document = window.document().expect("expecting a document on window");
        bulma::add_modals(&document);
    }
}

fn clear_cache() {
    LocalStorage::delete(TOKENS)
}

pub fn cache() -> gloo_storage::Result<HashMap<String, crate::models::Token>> {
    LocalStorage::get(TOKENS)
}

async fn request_metadata(mut token: crate::models::Token) -> Msg {
    let uri = token.url();
    match Request::get(&uri).send().await {
        Ok(response) => match response.status() {
            200 => {
                // Read response as text to handle empty result
                match response.text().await {
                    Ok(response) => {
                        if response.len() == 0 {
                            return Msg::NotFound;
                        }

                        match serde_json::from_str::<crate::metadata::Metadata>(&response) {
                            Ok(metadata) => {
                                token.metadata = Some(metadata);
                                Msg::Completed(token)
                            }
                            Err(e) => {
                                debug!(format!("{:?}", response));
                                error!(format!("{:?}", e));
                                Msg::Failed("An error occurred parsing the metadata".to_string())
                            }
                        }
                    }
                    Err(e) => {
                        error!(format!("{:?}", e));
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
                    error!(format!("{:?}", e));
                    Msg::Failed(format!("Requesting metadata from {uri} failed: {e}"))
                }
                _ => Msg::Failed(format!("Requesting metadata from {uri} failed: {e}")),
            }
        }
    }
}

#[function_component(RecentTokens)]
pub fn recent_tokens() -> yew::Html {
    use_effect(move || {
        // Attach carousel after component is rendered
        bulma::carousel::attach(Some("#recent-tokens"), Some(Options { slides_to_show: 4 }));
        || {}
    });
    let slides: Option<Vec<Html>> = cache().map_or(None, |recent_tokens| {
        Some(
            recent_tokens
                .values()
                .into_iter()
                .sorted_by_key(|token| token.last_viewed)
                .rev()
                .map(|token| {
                    let route = match token.id {
                        Some(id) => Route::CollectionToken {
                            uri: Uri::encode(&token.uri),
                            token: id,
                        },
                        None => Route::Token {
                            uri: Uri::encode(&token.uri),
                        },
                    };
                    match &token.metadata {
                        Some(metadata) => {
                            let name = metadata
                                .name
                                .as_ref()
                                .map_or("".to_string(), |name| name.clone());
                            html! {
                                <Link<Route> to={route}>
                                    <figure class="image">
                                        <img src={ metadata.image.clone() } alt={ name } />
                                    </figure>
                                </Link<Route>>
                            }
                        }
                        None => html! {},
                    }
                })
                .collect(),
        )
    });
    html! {
        if let Some(slides) = slides {
            <div id="recent-tokens">
                { slides }
            </div>
        }
    }
}

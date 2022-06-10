use crate::agents::Response;
use crate::components::token;
use crate::{cache, uri, Route};
use bulma::carousel::Options;
use gloo_net::http::Request;
use gloo_net::Error;
use itertools::Itertools;
use qrcode_generator::QrCodeEcc;
use std::rc::Rc;
use yew::prelude::*;
use yew_agent::{Bridge, Bridged};
use yew_router::prelude::*;

#[derive(Debug)]
pub enum Msg {
    Request(crate::models::Token),
    Redirect(String, crate::models::Token),
    Completed(crate::models::Token),
    Metadata(crate::metadata::Metadata),
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

#[derive(Debug)]
pub enum Status {
    NotStarted,
    Requesting,
    Completed,
    NotFound,
    Failed,
}

pub struct Token {
    agent: Box<dyn Bridge<crate::agents::Metadata>>,
    // The current token
    token: Option<crate::models::Token>,
    // If applicable, the corresponding collection
    collection: Option<crate::models::Collection>,
    // Any error, if applicable
    error: Option<String>,

    requesting: Option<crate::models::Token>,
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

    fn image(&self) -> Option<String> {
        self.token.as_ref().map_or(None, |token| {
            token.metadata.as_ref().map_or(None, |metadata| {
                if metadata.image.starts_with(".") {
                    Some(format!("{}{}", token.uri, &metadata.image))
                } else {
                    Some(metadata.image.clone())
                }
            })
        })
    }

    fn name(&self) -> String {
        self.token.as_ref().map_or("".to_string(), |token| {
            token.metadata.as_ref().map_or("".to_string(), |metadata| {
                metadata.name.as_ref().map_or(
                    // Use token number for missing name (if available)
                    match &self.collection {
                        None => token.id.map_or("".to_string(), |token| token.to_string()),
                        Some(collection) => token.id.map_or("".to_string(), |token| {
                            format!("{} {}", collection.name, token)
                        }),
                    },
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

        log::trace!("token: generating qr code...");
        match qrcode_generator::to_png_to_vec(&location, QrCodeEcc::Low, 128) {
            Ok(qr_code) => {
                log::trace!("token: qr code generated");
                Some(format!("data:image/png;base64,{}", base64::encode(qr_code)))
            }
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
}

impl Component for Token {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        let token = crate::models::Token::create(
            uri::decode(&ctx.props().token_uri).expect("unable to decode the uri"),
            ctx.props().token_id,
        );
        ctx.link().send_message(Msg::Request(token));
        let cb = {
            let link = ctx.link().clone();
            move |e| match e {
                Response::Completed(metadata) => {
                    link.send_message(Self::Message::Metadata(metadata))
                }
            }
        };
        let agent = crate::agents::Metadata::bridge(Rc::new(cb));

        Self {
            agent,
            token: None,
            collection: cache::Collection::get(&ctx.props().token_uri),
            error: None,

            requesting: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Request(token) => {
                self.error = None;

                // Check cache
                log::trace!("token: checking cache");
                let uri = token.url();
                if let Some(token) = cache::Token::get(&uri) {
                    ctx.link().send_message(Msg::Completed(token.clone()));
                    return true;
                }

                log::trace!("token: requesting metadata from agent");
                let url = token.url();
                self.agent
                    .send(crate::agents::Request::Metadata { uri: url.clone() });
                self.requesting = Some(token);
                // ctx.link()
                //     .send_future(async move { token::request_metadata(url, token).await });

                ctx.props().status.emit(Status::Requesting);
                true
            }
            Msg::Redirect(url, token) => {
                ctx.link()
                    .send_future(async move { token::request_metadata(url, token).await });
                self.error = None;
                ctx.props().status.emit(Status::Requesting);
                true
            }
            Msg::Completed(mut token) => {
                // Update token
                if let Some(mut metadata) = token.metadata.as_mut() {
                    if let Ok(token_uri) = uri::TokenUri::parse(&metadata.image, false) {
                        metadata.image = match token_uri.token {
                            None => token_uri.uri,
                            Some(id) => format!("{}{}", token_uri.uri, id),
                        }
                    }
                }
                ctx.props().status.emit(Status::Completed);
                self.token = Some(token.clone());

                // Cache token
                log::trace!("token: adding to cache");
                token.last_viewed = Some(chrono::offset::Utc::now());
                cache::Token::insert(token.url(), token);
                log::trace!("token: cached");
                true
            }
            Msg::Failed(error) => {
                ctx.props().status.emit(Status::Failed);
                self.error = Some(error);
                true
            }
            Msg::NotFound => {
                self.token = None;
                ctx.props().status.emit(Status::NotFound);
                true
            }
            Msg::Metadata(metadata) => {
                if let Some(mut token) = self.requesting.take() {
                    log::trace!("response completed {:?}", metadata);
                    token.metadata = Some(metadata);
                    ctx.link().send_message(Msg::Completed(token));
                }
                //ctx.link().send_message(Msg::Completed(metadata))
                false
            }
        }
    }

    fn changed(&mut self, ctx: &Context<Self>) -> bool {
        let uri = uri::decode(&ctx.props().token_uri).expect("unable to decode the uri");
        let id = ctx.props().token_id;
        if self.token.is_none()
            || uri != self.token.as_ref().unwrap().uri
            || id != self.token.as_ref().unwrap().id
        {
            log::trace!("token: token changed, requesting metadata...");
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
                            if let Some(image) = self.image() {
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
                                    <h1 class="title nifty-name">{ self.name() }</h1>
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
        log::trace!("token: rendered");
        // Wire up full screen image modal
        let window = web_sys::window().expect("global window does not exists");
        let document = window.document().expect("expecting a document on window");
        bulma::add_modals(&document);
    }
}

async fn request_metadata(uri: String, mut token: crate::models::Token) -> Msg {
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
                Some(uri) => Msg::Redirect(uri, token),
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

#[function_component(RecentTokens)]
pub fn recent_tokens() -> yew::Html {
    use_effect(move || {
        // Attach carousel after component is rendered
        bulma::carousel::attach(Some("#recent-tokens"), Some(Options { slides_to_show: 4 }));
        || {}
    });
    let slides: Option<Vec<Html>> = cache::Token::values().map_or(None, |recent_tokens| {
        Some(
            recent_tokens
                .into_iter()
                .sorted_by_key(|token| token.last_viewed)
                .rev()
                .map(|token| {
                    let route = match token.id {
                        Some(id) => Route::CollectionToken {
                            uri: uri::encode(&token.uri),
                            token: id,
                        },
                        None => Route::Token {
                            uri: uri::encode(&token.uri),
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

use crate::{
    cache,
    components::{
        token,
        token::{Status, Token},
    },
    models, Address, Route,
};
use std::rc::Rc;
use std::str::FromStr;
use web_sys::Document;
use workers::etherscan::{Contract, Request, Response};
use workers::{Bridge, Bridged, ParseError, Url};
use yew::prelude::*;
use yew_router::prelude::*;

pub struct Collection {
    worker: Box<dyn Bridge<workers::etherscan::Worker>>,
    collection: Option<models::Collection>,
    tokens: Vec<crate::models::Token>,
    status: Option<MessageStatus>,
}

pub enum Message {
    RequestContract(Address),
    Contract(Contract),
    NoContract(Address),
    ContractFailed(Address, u8),
    RequestBaseUri(Address),
    BaseUri(String),
    RequestTokenUri(Address),
    TokenUri(String, u8),
    UriFailed,
    Index,
}

#[derive(PartialEq, Properties)]
pub struct Properties {
    /// The collection identifier (contract address or base64-encoded url).
    pub id: String,
    pub api_key: Option<String>,
}

impl Component for Collection {
    type Message = Message;
    type Properties = Properties;

    fn create(ctx: &Context<Self>) -> Self {
        // Check if collection already cached
        let collection = cache::Collection::get(&ctx.props().id);
        if let None = collection {
            // Check if identifier is an address
            if let Ok(address) = Address::from_str(&ctx.props().id) {
                ctx.link().send_message(Message::RequestContract(address));
            } else {
                // todo: initialise collection via url
            }
        }

        Self {
            worker: workers::etherscan::Worker::bridge(Rc::new({
                let link = ctx.link().clone();
                move |e: workers::etherscan::Response| {
                    link.send_message(match e {
                        Response::Contract(contract) => Self::Message::Contract(contract),
                        Response::NoContract(address) => Self::Message::NoContract(address),
                        Response::ContractFailed(address, attempts) => {
                            Self::Message::ContractFailed(address, attempts)
                        }
                        Response::BaseUri(base_uri) => Self::Message::BaseUri(base_uri),
                        Response::NoBaseUri(address) => Self::Message::RequestTokenUri(address),
                        Response::BaseUriFailed(address) => Self::Message::RequestTokenUri(address),
                        Response::TokenUri(token_uri, token) => {
                            Self::Message::TokenUri(token_uri, token)
                        }
                        Response::NoTokenUri(address) => Self::Message::UriFailed,
                        Response::TokenUriFailed(address) => Self::Message::UriFailed,
                    })
                }
            })),
            collection,
            tokens: Vec::new(),
            status: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            // Contract
            Message::RequestContract(address) => {
                // Request contract info via etherscan worker
                self.worker.send(Request::Contract(address));
                self.status = Some(MessageStatus::Info(format!(
                    "Checking if address {} is a contract via etherscan.io...",
                    address
                )));
                true
            }
            Message::Contract(contract) => {
                self.collection = Some(models::Collection {
                    name: contract.name.clone(),
                    address: Some(ctx.props().id.clone()),
                    start_token: 1, // Default to one rather than zero to minimize failed contract calls
                    base_uri: None,
                });
                self.status = None;
                log::trace!("attempting to resolve first token using contract base uri ...");
                ctx.link()
                    .send_message(Message::RequestBaseUri(contract.address));
                true
            }
            Message::NoContract(address) => {
                self.status = Some(MessageStatus::Danger(format!(
                    "No contract found for {address}"
                )));
                true
            }
            Message::ContractFailed(address, attempts) => {
                self.status = Some(MessageStatus::Danger(format!(
                    "Contract could not be found for {address}, despite {attempts} attempts"
                )));
                true
            }
            // Base URI
            Message::RequestBaseUri(address) => {
                // Request contract info via etherscan worker
                self.worker.send(Request::BaseUri(address));
                false
            }
            Message::BaseUri(base_uri) => {
                if let Some(collection) = self.collection.as_mut() {
                    collection.base_uri = Some(base_uri);
                    ctx.link().send_message(Message::Index);
                    return true;
                }
                false
            }
            // Token URI
            Message::RequestTokenUri(address) => {
                // Request contract info via etherscan worker
                self.worker.send(Request::TokenUri(
                    address,
                    self.collection.as_ref().map_or(1, |c| c.start_token),
                ));
                false
            }
            Message::TokenUri(token_uri, token) => {
                if let Some(collection) = self.collection.as_mut() {
                    // Parse url to remove the final path segment (token) to use as base uri
                    match Url::from_str(&token_uri) {
                        Ok(url) => {
                            if let Some(segments) = url.path_segments() {
                                if let Some(token) = segments.last() {
                                    if let Some(base_uri) = token_uri.strip_suffix(token) {
                                        collection.base_uri = Some(base_uri.to_string());
                                        ctx.link().send_message(Message::Index);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("unable to parse the url '{token_uri}': {e:?}");
                            self.status = Some(MessageStatus::Danger(
                                "Could not determine the collection url".to_string(),
                            ));
                        }
                    }
                    return true;
                }
                false
            }
            Message::UriFailed => {
                self.status = Some(MessageStatus::Danger(
                    "Unable to determine the collection url via etherscan.io".to_string(),
                ));
                true
            }
            // Index
            Message::Index => {
                todo!("start indexing collection via metadata worker");
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let status = self.status.as_ref().map(|s| match s {
            MessageStatus::Info(message) => ("is-info", message),
            MessageStatus::Success(message) => ("is-success", message),
            MessageStatus::Warning(message) => ("is-warning", message),
            MessageStatus::Danger(message) => ("is-danger", message),
        });
        html! {
            <section class="section is-fullheight">
                if let Some(status) = status {
                    <article class={ classes!("message", status.0) }>
                        <div class="message-body">
                            { status.1.clone() }
                        </div>
                    </article>

                    if let None = ctx.props().api_key {
                        <article class="message is-danger">
                            <div class="message-body">
                                { format!("Note: No API key has been configured for the etherscan.io API. Requests are \
                                therefore throttled.",
                                ) }
                            </div>
                        </article>
                    }
                }

                if let Some(collection) = &self.collection {
                    <p>{ collection.name.clone() }</p>
                    <p>{ collection.base_uri.as_ref().map_or("".to_string(), |uri| uri.clone()) }</p>
                }

                <div class="columns is-multiline is-mobile">
                    <div class="column is-one-quarter">
                        <code>{"is-one-quarter"}</code>
                    </div>
                </div>
            </section>
        }
    }
}

enum MessageStatus {
    Info(String),
    Success(String),
    Warning(String),
    Danger(String),
}

pub struct CollectionToken {
    //_listener: HistoryListener,
    base_uri: String,
    start_token: usize,
    error: Option<String>,
    requesting_metadata: bool,
    document: Document,
    token_status_callback: Callback<token::Status>,
    token_status: token::Status,
}

pub enum CollectionTokenMessage {
    Navigated,
    TokenStatus(token::Status),
}

#[derive(PartialEq, Properties)]
pub struct CollectionTokenProperties {
    pub uri: String,
    pub token: usize,
}

impl Component for CollectionToken {
    type Message = CollectionTokenMessage;
    type Properties = CollectionTokenProperties;

    fn create(_ctx: &Context<Self>) -> Self {
        let window = web_sys::window().expect("global window does not exists");
        let document = window.document().expect("expecting a document on window");

        //let link = _ctx.link().clone();
        // let listener = _ctx.link().history().unwrap().listen(move || {
        //     link.send_message(Msg::Navigated);
        // });

        Self {
            //_listener: listener,
            base_uri: _ctx.props().uri.to_string(),
            start_token: 0,
            error: None,
            requesting_metadata: false,
            document,
            //token_uri: format!("{}{}", &_ctx.props().uri, _ctx.props().token),
            token_status_callback: _ctx
                .link()
                .callback(|status| CollectionTokenMessage::TokenStatus(status)),
            token_status: Status::NotStarted,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            CollectionTokenMessage::Navigated => {
                // let location = ctx.link().location().unwrap();
                // let route = location.route::<Route>().unwrap();
                // if let Route::CollectionToken { uri, token } = route {
                //     //self.token_uri = format!("{}{token}", uri::Uri::decode(&uri).unwrap());
                // }
                true
            }
            CollectionTokenMessage::TokenStatus(status) => {
                if matches!(status, Status::NotFound) && ctx.props().token == 0 {
                    let uri = &ctx.props().uri;
                    let start_token = ctx.props().token + 1;
                    if let Some(mut collection) = cache::Collection::get(&uri) {
                        collection.start_token = start_token as u8;
                        cache::Collection::insert(uri.clone(), collection);
                    }
                    ctx.link().history().unwrap().push(Route::CollectionToken {
                        uri: uri.clone(),
                        token: start_token,
                    });
                    return false;
                }

                self.token_status = status;
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let status = self.token_status_callback.clone();
        let token = ctx.props().token;

        html! {
            <section class="section is-fullheight">
                if let Some(error) = &self.error {
                    <div class="notification is-danger">
                      { error }
                    </div>
                }

                // todo: raise up to navigation to optimise space
                <div class="level is-mobile">
                    <div class="level-left"></div>
                    <div class="level-right">
                        <div class="field has-addons">
                          <div class="control">
                            if token > 0 {
                                <Link<Route> classes="button is-primary" to={Route::CollectionToken {
                                    uri: self.base_uri.clone(), token: token - 1 }}
                                    disabled={ self.requesting_metadata || token == self.start_token }>
                                    <span class="icon is-small">
                                      <i class="fas fa-angle-left"></i>
                                    </span>
                                </Link<Route>>
                            }
                          </div>
                          <div class="control">
                            <Link<Route> classes="button is-primary" to={Route::CollectionToken {
                                uri: self.base_uri.clone(), token: token + 1 }}
                                disabled={ self.requesting_metadata }>
                                <span class="icon is-small">
                                  <i class="fas fa-angle-right"></i>
                                </span>
                            </Link<Route>>
                          </div>
                        </div>
                    </div>
                </div>

                <Token token_uri={ self.base_uri.clone() } token_id={ ctx.props().token } {status} />

                if matches!(self.token_status, Status::NotFound) && ctx.props().token != self.start_token {
                    <article class="message is-primary">
                        <div class="message-body">
                            {"The requested token was not found. Have you reached the end of the collection? Click "}
                            <Link<Route> to={Route::CollectionToken {
                                uri: self.base_uri.clone(), token: self.start_token }}>
                                {"here"}
                            </Link<Route>>
                            {" to return to the start of the collection."}
                        </div>
                    </article>
                }
            </section>
        }
    }

    fn rendered(&mut self, _ctx: &Context<Self>, _first_render: bool) {
        // Wire up full screen image modal
        bulma::add_modals(&self.document);
    }
}

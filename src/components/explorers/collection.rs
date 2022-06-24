use crate::{
    cache,
    components::{
        token,
        token::{Status, Token},
    },
    models, uri, Address, Route,
};
use indexmap::IndexMap;
use std::rc::Rc;
use std::str::FromStr;
use web_sys::Document;
use workers::{etherscan, metadata, Bridge, Bridged, Url};
use yew::prelude::*;
use yew_router::prelude::*;

pub struct Collection {
    etherscan: Box<dyn Bridge<etherscan::Worker>>,
    metadata: Box<dyn Bridge<metadata::Worker>>,
    collection: Option<models::Collection>,
    tokens: Vec<models::Token>,
    status: Option<MessageStatus>,
}

pub enum Message {
    // Contract
    RequestContract(Address),
    Contract(etherscan::Contract),
    NoContract(Address),
    ContractFailed(Address, u8),
    // URI
    RequestUri(Address),
    Uri(String, Option<u32>),
    UriFailed,
    // Total Supply
    RequestTotalSupply(Address),
    TotalSupply(u32),
    // Metadata
    RequestMetadata(u32),
    Metadata(u32, metadata::Metadata),
    NotFound(u32),
    MetadataFailed(u32),
    // Ignore
    None,
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
        // Check if collection already exists locally
        let mut collection = cache::Collection::get(&ctx.props().id);
        match collection.as_mut() {
            None => {
                // Check if identifier is an address
                if let Ok(address) = Address::from_str(&ctx.props().id) {
                    ctx.link().send_message(Message::RequestContract(address));
                } else {
                    // todo: initialise collection via url
                }
            }
            Some(collection) => {
                if let Some(address) = collection.address.as_ref() {
                    // Check if base uri missing
                    match collection.base_uri.as_ref() {
                        None => ctx
                            .link()
                            .send_message(Message::RequestUri(address.clone())),
                        Some(_) => ctx
                            .link()
                            .send_message(Message::RequestMetadata(collection.start_token)),
                    }

                    // Check if total supply missing
                    if let None = collection.total_supply {
                        ctx.link()
                            .send_message(Message::RequestTotalSupply(address.clone()))
                    }
                }

                // Update last viewed on collection
                collection.last_viewed = Some(chrono::offset::Utc::now());
                cache::Collection::insert(ctx.props().id.clone(), collection.clone())
            }
        }

        Self {
            etherscan: etherscan::Worker::bridge(Rc::new({
                let link = ctx.link().clone();
                move |e: etherscan::Response| {
                    link.send_message(match e {
                        etherscan::Response::Contract(contract) => Message::Contract(contract),
                        etherscan::Response::NoContract(address) => Message::NoContract(address),
                        etherscan::Response::ContractFailed(address, attempts) => {
                            Message::ContractFailed(address, attempts)
                        }
                        etherscan::Response::Uri(uri, token) => Message::Uri(uri, token),
                        etherscan::Response::NoUri(_address) => Message::UriFailed,
                        etherscan::Response::UriFailed(address) => Message::UriFailed,
                        etherscan::Response::TotalSupply(total_supply) => {
                            Message::TotalSupply(total_supply)
                        }
                        etherscan::Response::NoTotalSupply(_) => Message::None,
                        etherscan::Response::TotalSupplyFailed(_) => Message::None,
                    })
                }
            })),
            metadata: metadata::Worker::bridge(Rc::new({
                let link = ctx.link().clone();
                move |e: metadata::Response| match e {
                    metadata::Response::Completed(metadata, token) => link.send_message(
                        Message::Metadata(token.expect("expected valid token"), metadata),
                    ),
                    metadata::Response::NotFound(url, token) => {
                        link.send_message(Message::NotFound(token.expect("expected valid token")))
                    }
                    metadata::Response::Failed(url, token) => link.send_message(
                        Message::MetadataFailed(token.expect("expected valid token")),
                    ),
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
                self.etherscan.send(etherscan::Request::Contract(address));
                self.status = Some(MessageStatus::Info(format!(
                    "Checking if address {} is a contract via etherscan.io...",
                    address
                )));
                true
            }
            Message::Contract(contract) => {
                let collection = models::Collection {
                    address: Some(contract.address),
                    name: contract.name.clone(),
                    base_uri: None,
                    start_token: 0,
                    total_supply: None,
                    tokens: IndexMap::new(),
                    last_viewed: Some(chrono::offset::Utc::now()),
                };
                cache::Collection::insert(ctx.props().id.clone(), collection.clone());
                self.collection = Some(collection);
                self.status = None;
                log::trace!("attempting to resolve uri from contract ...");
                ctx.link()
                    .send_message(Message::RequestUri(contract.address));
                log::trace!("attempting to resolve total supply from contract ...");
                ctx.link()
                    .send_message(Message::RequestTotalSupply(contract.address));
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
            // URI
            Message::RequestUri(address) => {
                // Request contract info via etherscan worker
                self.etherscan.send(etherscan::Request::Uri(
                    address,
                    1, // Default to one rather than zero to minimize failed contract calls
                ));
                false
            }
            Message::Uri(uri, token) => {
                if let Some(collection) = self.collection.as_mut() {
                    match uri::parse(&uri) {
                        Ok(url) => {
                            // Check if url contains token
                            match token {
                                Some(_) => {
                                    // Parse url to remove the final path segment (token) to use as base uri
                                    if let Some(base_uri) = url
                                        .path_segments()
                                        .and_then(|segments| segments.last())
                                        .and_then(|token| url.as_str().strip_suffix(token))
                                    {
                                        collection.base_uri = Some(
                                            Url::from_str(base_uri).expect("expected a valid url"),
                                        );
                                    }
                                }
                                None => {
                                    collection.base_uri = Some(url);
                                }
                            }

                            cache::Collection::insert(ctx.props().id.clone(), collection.clone());
                            // Request first item in collection
                            ctx.link()
                                .send_message(Message::RequestMetadata(collection.start_token));
                            return true;
                        }
                        Err(e) => {
                            log::error!("unable to parse the url '{uri}': {e:?}");
                            self.status = Some(MessageStatus::Danger(
                                "Could not determine the collection url".to_string(),
                            ));
                        }
                    }
                }
                false
            }
            Message::UriFailed => {
                self.status = Some(MessageStatus::Danger(
                    "Unable to determine the collection url via etherscan.io".to_string(),
                ));
                true
            }
            // Total Supply
            Message::RequestTotalSupply(address) => {
                // Request contract info via etherscan worker
                self.etherscan
                    .send(etherscan::Request::TotalSupply(address));
                false
            }
            Message::TotalSupply(total_supply) => {
                if let Some(collection) = self.collection.as_mut() {
                    collection.total_supply = Some(total_supply);
                    cache::Collection::insert(ctx.props().id.clone(), collection.clone());
                }
                false
            }
            // Metadata
            Message::RequestMetadata(token) => {
                if let Some(collection) = self.collection.as_ref() {
                    // Check if token already exists
                    if collection.tokens.contains_key(&token) {
                        // Request next token
                        ctx.link().send_message(Message::RequestMetadata(token + 1))
                    } else if let Some(base_uri) = collection.base_uri.as_ref() {
                        self.metadata.send(metadata::Request {
                            url: format!("{base_uri}{token}"),
                            token: Some(token),
                            cors_proxy: Some(crate::config::CORS_PROXY.to_string()),
                        })
                    }
                }
                false
            }
            Message::Metadata(token, metadata) => {
                if let Some(collection) = self.collection.as_mut() {
                    // Add token to collection and request next item
                    collection.add(token, metadata);
                    cache::Collection::insert(ctx.props().id.clone(), collection.clone());
                    if token < 1000 {
                        ctx.link().send_message(Message::RequestMetadata(token + 1));
                    }
                    return true;
                }
                false
            }
            Message::NotFound(token) | Message::MetadataFailed(token) => {
                if let Some(collection) = self.collection.as_mut() {
                    if token == collection.start_token {
                        collection.start_token += 1;
                        ctx.link().send_message(Message::RequestMetadata(token + 1))
                    }
                }
                false
            }
            // Ignore
            Message::None => false,
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
                <h1 class="title nifty-name">{ collection.name.clone() }</h1>
                if let Some(address) = collection.address.as_ref() {
                    <h1 class="subtitle">{ address.to_string() }</h1>
                }
                if let Some(total_supply) = collection.total_supply.as_ref() {
                    <h1 class="subtitle">{ total_supply.to_string() }</h1>
                }

                <div class="columns is-multiline">
                {
                    collection.tokens.values().map(|token| {
                    html! {
                        if let Some(metadata) = token.metadata.as_ref() {
                            <div class="column is-2">
                                <Link<Route> to={ Route::token(&token, Some(collection)) }>
                                    <figure class="image">
                                        <img src={ metadata.image.clone() } alt={ metadata.name.clone() } />
                                    </figure>
                                </Link<Route>>
                            </div>
                        }
                    }
                    }).collect::<Html>()
                }
                </div>
            }
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
    start_token: u32,
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
    pub token: u32,
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
                        collection.start_token = start_token;
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

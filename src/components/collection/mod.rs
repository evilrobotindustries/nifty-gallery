use crate::storage::Get;
use crate::{models, notifications, storage, uri, Address, Route, Scroll};
use bulma::toast::Color;
use std::rc::Rc;
use std::str::FromStr;
use thousands::Separable;
use workers::etherscan::TypeExtensions;
use workers::metadata::Metadata;
use workers::{etherscan, metadata, Bridge, Bridged, Url};
use yew::prelude::*;
use yew_router::prelude::*;

pub mod token;

pub struct Collection {
    etherscan: Box<dyn Bridge<etherscan::Worker>>,
    metadata: Box<dyn Bridge<metadata::Worker>>,
    collection: Option<models::Collection>,
    tokens: Vec<models::Token>,
    notified_indexing: bool,
    indexed: usize,
    page: usize,
    page_size: usize,
    working: bool,
}

pub enum Message {
    // Contract
    MissingApiKey,
    RequestContract(Address),
    Contract(etherscan::Contract),
    NoContract(Address),
    ContractFailed(Address, u8),
    CopyAddress,
    // URI
    RequestUri(Address),
    Uri(String, Option<u32>),
    UriFailed,
    // Total Supply
    RequestTotalSupply(Address),
    TotalSupply(u32),
    // Metadata
    RequestMetadata(u32),
    Metadata(String, u32, Metadata),
    NotFound(u32),
    MetadataFailed(u32),
    // Paging
    Page(usize),
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
        let mut collection = storage::Collection::get(ctx.props().id.as_str());
        match collection.as_mut() {
            None => {
                // Check if identifier is an address
                if let Ok(address) = Address::from_str(&ctx.props().id) {
                    collection = Some(models::Collection::Contract {
                        address,
                        name: TypeExtensions::format(&address),
                        base_uri: None,
                        start_token: 0,
                        total_supply: None,
                        last_viewed: None,
                    });

                    if let None = ctx.props().api_key {
                        ctx.link().send_message(Message::MissingApiKey);
                    }

                    ctx.link().send_message(Message::RequestContract(address));
                } else {
                    // Initialise collection from url
                    match uri::decode(ctx.props().id.as_str()) {
                        Ok(url) => match uri::parse(url.as_str()) {
                            Ok(base_uri) => {
                                let c = models::Collection::Url {
                                    id: ctx.props().id.clone(),
                                    base_uri: Some(base_uri),
                                    start_token: 0,
                                    total_supply: None,
                                    last_viewed: None,
                                };
                                storage::Collection::store(c.clone());
                                collection = Some(c);
                                ctx.link().send_message(Message::RequestMetadata(0))
                            }
                            Err(e) => {
                                log::error!("unable to parse the collection url '{url}': {e:?}")
                            }
                        },
                        Err(e) => {
                            log::error!(
                                "unable to decode the collection identifier '{}': {e:?}",
                                ctx.props().id
                            )
                        }
                    }
                }
            }
            Some(collection) => {
                match collection {
                    models::Collection::Contract {
                        address,
                        base_uri,
                        total_supply,
                        start_token,
                        ..
                    } => {
                        // Check if base uri missing
                        match base_uri.as_ref() {
                            None => ctx
                                .link()
                                .send_message(Message::RequestUri(address.clone())),
                            Some(_) => ctx
                                .link()
                                .send_message(Message::RequestMetadata(start_token.clone())),
                        }

                        // Check if total supply missing
                        if let None = total_supply {
                            ctx.link()
                                .send_message(Message::RequestTotalSupply(address.clone()))
                        }
                    }
                    models::Collection::Url { start_token, .. } => ctx
                        .link()
                        .send_message(Message::RequestMetadata(start_token.clone())),
                }

                // Initialise first page
                ctx.link().send_message(Message::Page(1));

                // Update last viewed on collection and store
                collection.set_last_viewed();
                storage::Collection::store(collection.clone())
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
                        etherscan::Response::UriFailed(_address) => Message::UriFailed,
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
                    metadata::Response::Completed(url, token, metadata) => link.send_message(
                        Message::Metadata(url, token.expect("expected valid token"), metadata),
                    ),
                    metadata::Response::NotFound(_url, token) => {
                        link.send_message(Message::NotFound(token.expect("expected valid token")))
                    }
                    metadata::Response::Failed(_url, token) => link.send_message(
                        Message::MetadataFailed(token.expect("expected valid token")),
                    ),
                }
            })),
            collection,
            tokens: Vec::new(),
            notified_indexing: false,
            indexed: 0,
            page: 1,
            page_size: 25,
            working: false,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            // Contract
            Message::MissingApiKey => {
                notifications::notify(
                    "Warning: No API key has been configured for the etherscan.io API. Requests are therefore throttled.".to_string(),
                    Some(Color::Warning),
                );
                false
            }
            Message::RequestContract(address) => {
                // Request contract info via etherscan worker
                self.etherscan.send(etherscan::Request::Contract(address));
                notifications::notify(
                    format!(
                        "Checking if address {} is a contract via etherscan.io...",
                        address
                    ),
                    None,
                );
                self.working = true;
                true
            }
            Message::Contract(contract) => {
                // Initialise collection from contract
                let collection = match storage::Collection::get(&contract.address) {
                    None => models::Collection::Contract {
                        address: contract.address,
                        name: contract.name.clone(),
                        base_uri: None,
                        start_token: 0,
                        total_supply: None,
                        last_viewed: Some(chrono::offset::Utc::now()),
                    },
                    Some(collection) => collection,
                };

                // Check if collection missing any data which can be resolved from contract
                self.working = false;
                if let models::Collection::Contract {
                    address,
                    base_uri,
                    total_supply,
                    ..
                } = &collection
                {
                    if let None = base_uri {
                        log::trace!("attempting to resolve uri from contract ...");
                        ctx.link()
                            .send_message(Message::RequestUri(address.clone()));
                        self.working = true;
                    }
                    if let None = total_supply {
                        log::trace!("attempting to resolve total supply from contract ...");
                        ctx.link()
                            .send_message(Message::RequestTotalSupply(address.clone()));
                        self.working = true;
                    }
                }

                // Store collection locally
                storage::Collection::store(collection.clone());
                self.collection = Some(collection);
                true
            }
            Message::NoContract(address) => {
                notifications::notify(
                    format!("No contract found for {address}"),
                    Some(Color::Danger),
                );
                self.working = false;
                true
            }
            Message::ContractFailed(address, attempts) => {
                notifications::notify(
                    format!(
                        "Contract could not be found for {address}, despite {attempts} attempts"
                    ),
                    Some(Color::Danger),
                );
                self.working = false;
                true
            }
            Message::CopyAddress => {
                if let Some(models::Collection::Contract { address, .. }) = self.collection {
                    let window = web_sys::window().expect("global window does not exists");
                    if let Some(clipboard) = window.navigator().clipboard() {
                        let _ = clipboard.write_text(&TypeExtensions::format(&address));
                    }
                }
                false
            }
            // URI
            Message::RequestUri(address) => {
                // Request contract info via etherscan worker
                self.etherscan.send(etherscan::Request::Uri(
                    address,
                    1, // Default to one rather than zero to minimize failed contract calls
                ));
                self.working = true;
                true
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
                                        collection.set_base_uri(
                                            Url::from_str(base_uri).expect("expected a valid url"),
                                        );
                                    }
                                }
                                None => {
                                    collection.set_base_uri(url);
                                }
                            }
                            storage::Collection::store(collection.clone());

                            // Request first item in collection
                            ctx.link().send_message(Message::RequestMetadata(
                                collection.start_token().clone(),
                            ));
                            return true;
                        }
                        Err(e) => {
                            log::error!("unable to parse the url '{uri}': {e:?}");
                            notifications::notify(
                                "Could not determine the collection url".to_string(),
                                Some(Color::Danger),
                            );
                        }
                    }
                }
                self.working = false;
                true
            }
            Message::UriFailed => {
                notifications::notify(
                    "Unable to determine the collection url via etherscan.io. Please try again..."
                        .to_string(),
                    Some(Color::Danger),
                );
                self.working = false;
                true
            }
            // Total Supply
            Message::RequestTotalSupply(address) => {
                // Request contract info via etherscan worker
                self.etherscan
                    .send(etherscan::Request::TotalSupply(address));
                self.working = true;
                true
            }
            Message::TotalSupply(total_supply) => {
                if let Some(collection) = self.collection.as_mut() {
                    collection.set_total_supply(total_supply);
                    storage::Collection::store(collection.clone());
                }
                self.working = false;
                false
            }
            // Metadata
            Message::RequestMetadata(token) => {
                // Check if token already exists in current view
                if self.tokens.iter().any(|t| t.id == token) {
                    // Request next token
                    ctx.link().send_message(Message::RequestMetadata(token + 1));
                } else {
                    if let Some(collection) = self.collection.as_ref() {
                        // Check if token already exists within storage
                        if let Some(_token) = storage::Token::get(collection.id().as_str(), token) {
                            // Request next token
                            ctx.link().send_message(Message::RequestMetadata(token + 1));
                        }
                        // Otherwise request metadata
                        else if let Some(url) = collection.url(token) {
                            self.metadata.send(metadata::Request {
                                url,
                                token: Some(token),
                                cors_proxy: Some(crate::config::CORS_PROXY.to_string()),
                            });
                            self.working = true;
                            return true;
                        }
                    }
                }

                false
            }
            Message::Metadata(url, token, metadata) => {
                // Ignore any metadata returned from worker which doesnt pertain to current collection
                if !url.starts_with(
                    self.collection
                        .as_ref()
                        .and_then(|c| c.base_uri().as_ref())
                        .map_or_else(|| "", |base_uri| base_uri.as_str()),
                ) {
                    log::trace!(
                        "received token {token} at {url} does not match currently viewed collection {}",
                        ctx.props().id
                    );
                    return false;
                }

                self.working = false;
                // Add token to collection and request next item
                self.add(token, metadata);
                if !self.notified_indexing {
                    let message = if url.contains("ipfs") {
                        "Indexing collection from IPFS, this may take some time..."
                    } else {
                        "Indexing collection..."
                    };
                    notifications::notify(message.to_string(), None);
                    self.notified_indexing = true;
                }

                ctx.link().send_message(Message::RequestMetadata(token + 1));
                self.working = true;
                true
            }
            Message::NotFound(token) | Message::MetadataFailed(token) => {
                self.working = false;
                if let Some(collection) = self.collection.as_mut() {
                    if token == *collection.start_token() {
                        collection.increment_start_token(1);
                        ctx.link().send_message(Message::RequestMetadata(token + 1));
                        return false;
                    }
                    match collection.total_supply() {
                        Some(total_supply) => {
                            // Continue indexing until total supply reached
                            if token < *total_supply {
                                ctx.link().send_message(Message::RequestMetadata(token + 1))
                            }
                        }
                        None => {
                            // Continue indexing for a maximum of 100 tokens
                            if token < 100 {
                                ctx.link().send_message(Message::RequestMetadata(token + 1))
                            }
                        }
                    }
                }
                true
            }
            // Paging
            Message::Page(page) => {
                self.page = page;

                if let Some(collection) = self.collection.as_ref() {
                    let (page, total) =
                        storage::Token::page(collection.id().as_str(), page - 1, self.page_size);
                    self.tokens = page;
                    self.indexed = total;
                }

                true
            }
            // Ignore
            Message::None => false,
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let page = self.page;
        let copy_address = ctx.link().callback(move |_| Message::CopyAddress);
        let previous_page = ctx.link().callback(move |_| {
            if let Some(window) = web_sys::window() {
                Scroll::top(&window);
            }
            Message::Page(page - 1)
        });
        let next_page = ctx.link().callback(move |_| {
            if let Some(window) = web_sys::window() {
                Scroll::top(&window);
            }
            Message::Page(page + 1)
        });
        let image_onload = Callback::from(move |e: web_sys::Event| {
            if let Some(figure) = e
                .target_unchecked_into::<web_sys::HtmlElement>()
                .offset_parent()
            {
                let _ = figure.class_list().remove_1("is-square");
            }
        });

        html! {
            <div id="collection">
            if let Some(collection) = &self.collection {
                <section class="section is-header">
                    <div class="columns">
                        <div class="column">
                            if let Some(name) = collection.name() {
                                <h1 class="title nifty-name">{ name.clone() }</h1>
                            }
                            <div class="level is-mobile">
                                <div class="level-left">
                                    if let models::Collection::Contract{ address, ..} = collection {
                                        <div class="level-item no-space">
                                            <p class="has-tooltip-right" data-tooltip={ TypeExtensions::format(address) } >
                                                { address.to_string() }
                                            </p>
                                        </div>
                                        <div class="level-item">
                                            <button onclick={ copy_address } class="button">
                                                <span class="icon is-small">
                                                  <i class="fa-regular fa-clone"></i>
                                                </span>
                                            </button>
                                        </div>
                                    }
                                    <span class="level-item">
                                        { self.indexed.separate_with_commas() }
                                        if let Some(total_supply) = collection.total_supply() {
                                            {" / "}{ total_supply.separate_with_commas() }
                                        }
                                        {" items"}
                                    </span>
                                    if self.working {
                                        <i class="is-loading level-item"></i>
                                    }
                                </div>
                            </div>
                        </div>
                        <div class="column">
                            <Navigate { page } page_size={ self.page_size } items={ self.indexed }
                                previous={ previous_page.clone() } next={ next_page.clone() } />
                        </div>
                    </div>
                </section>

                // Collection page
                <section class="section">
                    <div class="columns is-multiline">{ self.tokens.iter().filter_map(|token| token.metadata.as_ref()
                        .map(|metadata| html! {
                            <div class="column is-one-fifth">
                                <Link<Route> to={ Route::token(token, collection.id()) }>
                                    <figure class="image is-square">
                                        <img src={ metadata.image.clone() } alt={ metadata.name.clone() }
                                             onload={ image_onload.clone() } />
                                    </figure>
                                </Link<Route>>
                            </div>
                        })).collect::<Html>()  }
                    </div>
                </section>
            }
            </div>
        }
    }
}

impl Collection {
    pub fn add(&mut self, id: u32, mut metadata: Metadata) {
        // Parse urls
        metadata.image = uri::parse(&metadata.image).map_or(metadata.image, |url| url.to_string());
        if let Some(animation_url) = &metadata.animation_url {
            metadata.animation_url = uri::parse(&animation_url)
                .map_or(metadata.animation_url, |url| Some(url.to_string()));
        }

        if let Some(collection) = self.collection.as_ref() {
            let token = models::Token {
                id,
                metadata: Some(metadata),
                last_viewed: None,
            };

            self.indexed = storage::Token::store(collection.id().as_str(), token.clone());

            let page_start = ((self.page - 1) * self.page_size) as u32 + *collection.start_token();
            let page_end = page_start + self.page_size as u32;
            if token.id >= page_start && token.id < page_end {
                self.tokens.push(token);
            }
        }
    }
}

#[derive(Properties, PartialEq)]
struct NavigateProps {
    page: usize,
    page_size: usize,
    items: usize,
    previous: Callback<MouseEvent>,
    next: Callback<MouseEvent>,
}

#[function_component(Navigate)]
fn navigate(props: &NavigateProps) -> Html {
    html! {
        <div class="level is-mobile is-bottom">
            <div class="level-left"></div>
            <div class="level-right">
                <div class="field has-addons">
                  <div class="control">
                    if props.page > 1 {
                        <button onclick={ &props.previous } class="button is-primary">
                            <span class="icon is-small">
                              <i class="fas fa-angle-left"></i>
                            </span>
                        </button>
                    }
                  </div>
                  <div class="control">
                    if props.page * props.page_size < props.items {
                        <button onclick={ &props.next } class="button is-primary">
                            <span class="icon is-small">
                              <i class="fas fa-angle-right"></i>
                            </span>
                        </button>
                    }
                  </div>
                </div>
            </div>
        </div>
    }
}

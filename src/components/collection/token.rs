use crate::components::collection::Collection;
use crate::storage::RecentlyViewedItem;
use crate::{
    components::token, models, notifications, notifications::Color, storage, storage::Get, uri,
    Address, Route,
};
use std::rc::Rc;
use std::str::FromStr;
use workers::metadata::Metadata;
use workers::{etherscan, metadata, Bridge, Bridged, Url};
use yew::prelude::*;
use yew_router::prelude::*;

/// A token within a collection.
pub struct Token {
    etherscan: Box<dyn Bridge<etherscan::Worker>>,
    metadata: Box<dyn Bridge<metadata::Worker>>,
    collection: Option<models::Collection>,
    token: Option<models::Token>,
    notified_requesting_metadata: bool,
    working: bool,
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
    Metadata(String, u32, Metadata),
    NotFound(u32),
    MetadataFailed(u32),
    // Viewed
    Viewed(String, u32, String, String),
    // Ignore
    None,
}

#[derive(PartialEq, Properties)]
pub struct Properties {
    /// The collection identifier.
    pub collection: String,
    /// The token identifier.
    pub token: u32,
}

impl Component for Token {
    type Message = Message;
    type Properties = Properties;

    fn create(ctx: &Context<Self>) -> Self {
        let mut collection = storage::Collection::get(ctx.props().collection.as_str());
        let token = storage::Token::get(
            ctx.props().collection.as_str(),
            Collection::calculate_page(ctx.props().token),
            ctx.props().token,
        );

        match collection.as_ref() {
            None => {
                // Check if identifier is an address
                if let Ok(address) = Address::from_str(ctx.props().collection.as_str()) {
                    ctx.link().send_message(Message::RequestContract(address));
                } else {
                    // Initialise collection from url
                    match uri::decode(ctx.props().collection.as_str()) {
                        Ok(url) => match uri::parse(url.as_str()) {
                            Ok(base_uri) => {
                                let c = models::Collection::Url {
                                    id: ctx.props().collection.clone(),
                                    base_uri: Some(base_uri),
                                    start_token: 0,
                                    total_supply: None,
                                    last_viewed: None,
                                };
                                storage::Collection::store(c.clone());
                                collection = Some(c);
                                ctx.link()
                                    .send_message(Message::RequestMetadata(ctx.props().token))
                            }
                            Err(e) => {
                                log::error!("unable to parse the collection url '{url}': {e:?}")
                            }
                        },
                        Err(e) => {
                            log::error!(
                                "unable to decode the collection identifier '{}': {e:?}",
                                ctx.props().collection
                            )
                        }
                    }
                }
            }
            Some(collection) => {
                if let None = collection.base_uri() {
                    if let models::Collection::Contract { address, .. } = collection {
                        ctx.link()
                            .send_message(Message::RequestUri(address.clone()))
                    }
                } else if let None = token {
                    ctx.link()
                        .send_message(Message::RequestMetadata(ctx.props().token))
                }
            }
        }
        if let Some(metadata) = token.as_ref().and_then(|t| t.metadata.as_ref()) {
            // Add to recently viewed
            ctx.link().send_message(Message::Viewed(
                ctx.props().collection.clone(),
                ctx.props().token,
                metadata
                    .name
                    .as_ref()
                    .unwrap_or(&ctx.props().token.to_string())
                    .to_string(),
                metadata.image.clone(),
            ));
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
            token,
            notified_requesting_metadata: false,
            working: false,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            // Contract
            Message::RequestContract(address) => {
                // Request contract info via etherscan worker
                self.etherscan.send(etherscan::Request::Contract(address));
                notifications::notify(
                    format!("Checking if address {address} is a contract via etherscan.io...",),
                    Some(Color::Info),
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

                            // Request current item
                            ctx.link()
                                .send_message(Message::RequestMetadata(ctx.props().token));
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
                // Check if token already exists
                log::trace!("checking if token {token} already exists locally...");
                match storage::Token::get(
                    ctx.props().collection.as_str(),
                    Collection::calculate_page(token),
                    token,
                ) {
                    None => {
                        if let Some(url) = self.collection.as_ref().and_then(|c| c.url(token)) {
                            if !self.notified_requesting_metadata {
                                let message = if url.contains("ipfs") {
                                    "Requesting metadata from IPFS, this may take some time..."
                                } else {
                                    "Requesting metadata..."
                                };
                                notifications::notify(message.to_string(), None);
                                self.notified_requesting_metadata = true;
                            }

                            log::trace!("requesting metadata for token {token} from {url}...");
                            self.metadata.send(metadata::Request {
                                url,
                                token: Some(token),
                                cors_proxy: Some(crate::config::CORS_PROXY.to_string()),
                            });
                            self.working = true;
                        }
                    }
                    Some(t) => {
                        // Add to recently viewed
                        if let Some(metadata) = &t.metadata {
                            log::trace!("adding token to recently viewed...");
                            ctx.link().send_message(Message::Viewed(
                                ctx.props().collection.clone(),
                                token,
                                metadata
                                    .name
                                    .as_ref()
                                    .unwrap_or(&token.to_string())
                                    .to_string(),
                                metadata.image.clone(),
                            ));
                        }

                        self.token = Some(t);
                        self.working = false;
                    }
                }

                true
            }
            Message::Metadata(url, token, metadata) => {
                // Ignore any metadata returned from worker which doesnt pertain to current token
                if Some(url)
                    != self
                        .collection
                        .as_ref()
                        .and_then(|c| c.url(ctx.props().token))
                {
                    log::trace!(
                        "received token {token} does not match currently viewed token {}",
                        ctx.props().token
                    );
                    return false;
                }

                // Add to recently viewed
                ctx.link().send_message(Message::Viewed(
                    ctx.props().collection.clone(),
                    token,
                    metadata
                        .name
                        .as_ref()
                        .unwrap_or(&token.to_string())
                        .to_string(),
                    metadata.image.clone(),
                ));

                // Initialise token
                let current_token = models::Token::new(token, metadata);
                storage::Token::store(
                    ctx.props().collection.as_str(),
                    Collection::calculate_page(token),
                    current_token.clone(),
                );
                self.token = Some(current_token);
                self.working = false;
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
            // Viewed
            Message::Viewed(collection, token, name, image) => {
                storage::RecentlyViewed::store(RecentlyViewedItem {
                    name,
                    image,
                    route: Route::CollectionToken {
                        id: collection,
                        token,
                    },
                });
                false
            }
            // Ignore
            Message::None => false,
        }
    }

    fn changed(&mut self, ctx: &Context<Self>) -> bool {
        match storage::Token::get(
            ctx.props().collection.as_str(),
            Collection::calculate_page(ctx.props().token),
            ctx.props().token,
        ) {
            None => {
                log::trace!("token changed, requesting metadata...");
                ctx.link()
                    .send_message(Message::RequestMetadata(ctx.props().token));
                false
            }
            Some(token) => {
                self.token = Some(token);
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let start_token = self.collection.as_ref().map_or(0, |c| *c.start_token());

        html! {
            <section id="piece" class="section is-fullheight">
                // Collection navigation
                <Navigate collection={ ctx.props().collection.clone() } token={ ctx.props().token }
                    working={ self.working } { start_token } />

                // Current Token
                if let Some(token) = self.token.as_ref() {
                    <token::Token token={ Rc::new(token.clone()) } />
                }

                // End of collection error
                // if matches!(self.token_status, Status::NotFound) && ctx.props().token != self.start_token {
                //     <article class="message is-primary">
                //         <div class="message-body">
                //             {"The requested token was not found. Have you reached the end of the collection? Click "}
                //             <Link<Route>
                //                 to={Route::CollectionToken { id: self.base_uri.clone(), token: self.start_token }}>
                //                 {"here"}
                //             </Link<Route>>
                //             {" to return to the start of the collection."}
                //         </div>
                //     </article>
                // }
            </section>
        }
    }
}

#[derive(Properties, PartialEq)]
struct NavigateProps {
    collection: String,
    token: u32,
    working: bool,
    start_token: u32,
}

#[function_component(Navigate)]
fn navigate(props: &NavigateProps) -> Html {
    html! {
        <div class="level is-mobile">
            <div class="level-left"></div>
            <div class="level-right">
                <div class="field has-addons">
                    if props.working {
                        <div class="control">
                            <a class="button">
                                <span class="icon is-small has-tooltip-bottom" data-tooltip="View Collection">
                                    <i class="is-loading"></i>
                                </span>
                            </a>
                        </div>
                    }
                    <div class="control">
                        <Link<Route> classes="button"
                            to={Route::Collection { id: props.collection.clone() }}>
                            <span class="icon is-small has-tooltip-bottom" data-tooltip="View Collection">
                                <i class="fa-solid fa-grip"></i>
                            </span>
                        </Link<Route>>
                    </div>
                    <div class="control">
                        if props.token > 0 {
                            <Link<Route> classes="button is-primary"
                                to={Route::CollectionToken { id: props.collection.clone(), token: props.token - 1 }}
                                disabled={ props.working || props.token == props.start_token }>
                                <span class="icon is-small">
                                    <i class="fas fa-angle-left"></i>
                                </span>
                            </Link<Route>>
                        }
                    </div>
                    <div class="control">
                        <Link<Route> classes="button is-primary"
                            to={Route::CollectionToken { id: props.collection.clone(), token: props.token + 1 }}
                            disabled={ props.working }>
                            <span class="icon is-small">
                                <i class="fas fa-angle-right"></i>
                            </span>
                        </Link<Route>>
                    </div>
                </div>
            </div>
        </div>
    }
}

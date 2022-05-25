use crate::components::token;
use crate::components::token::{Status, Token};
use crate::{uri, Route};
use etherscan::contracts::{Contract, ABI};
use etherscan::Tag;
use gloo_console::{debug, error};
use gloo_timers::future::sleep;
use std::str::FromStr;
use std::time::Duration;
use web_sys::Document;
use yew::prelude::*;
use yew_router::prelude::*;

const THROTTLE_SECONDS: u64 = 5;

pub struct Address {
    client: etherscan::Client,
    address: crate::Address,
    name: Option<String>,
    abi: Option<ABI>,
    status: Option<String>,
}

impl Address {
    fn prepare_call(&self) -> Option<(UriType, String, ethabi::Function)> {
        if let Some(contract) = &self.abi {
            if let Ok(base_uri) = contract.function("baseURI") {
                debug!(format!("{:?}", base_uri));
                if base_uri.inputs.len() == 0 {
                    if let Ok(encoded) = base_uri.encode_input(&vec![]) {
                        return Some((UriType::BaseUri, hex::encode(&encoded), base_uri.clone()));
                    }
                }
            }
            if let Ok(token_uri) = contract.function("tokenURI") {
                debug!(format!("{:?}", token_uri));
                if token_uri.inputs.len() == 1 {
                    if let Ok(encoded) =
                        token_uri.encode_input(&vec![ethabi::token::Token::Uint(0.into())])
                    {
                        return Some((UriType::TokenUri, hex::encode(&encoded), token_uri.clone()));
                    }
                }
            }
        }

        None
    }

    async fn call(
        api_key: String,
        address: etherscan::Address,
        function: (UriType, String, ethabi::Function),
    ) -> AddressMsg {
        let client = etherscan::proxy::Client::new(api_key);

        match client.call(&address, &function.1, Some(Tag::Latest)).await {
            Ok(result) => {
                let decoded = hex::decode(&result[2..]).expect("could not decoded the call result");
                match function.2.decode_output(&decoded) {
                    Ok(tokens) => {
                        return AddressMsg::UriResolved(function.0, tokens[0].to_string());
                    }
                    Err(e) => {
                        error!(format!("{:?}", e))
                    }
                }
            }
            Err(e) => {
                error!(format!("{:?}", e))
            }
        }
        AddressMsg::NoContract
    }
}

pub enum UriType {
    BaseUri,
    TokenUri,
}

pub enum AddressMsg {
    RequestContract,
    Contract(Contract),
    NoContract,
    ResolveUri,
    UriResolved(UriType, String),
}

#[derive(PartialEq, Properties)]
pub struct AddressProps {
    pub address: String,
    pub api_key: Option<String>,
}

impl Component for Address {
    type Message = AddressMsg;
    type Properties = AddressProps;

    fn create(_ctx: &Context<Self>) -> Self {
        _ctx.link().send_message(AddressMsg::RequestContract);

        let api_key = _ctx
            .props()
            .api_key
            .as_ref()
            .map_or("".to_string(), |k| k.clone());
        Self {
            client: etherscan::Client::new(api_key),
            address: crate::Address::from_str(&_ctx.props().address).unwrap(),
            name: None,
            abi: None,
            status: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            AddressMsg::RequestContract => {
                let client = etherscan::contracts::Client::from(self.client.clone());
                let address = self.address;
                ctx.link().send_future(async move {
                    if let Ok(mut contracts) = client.get_source_code(&address).await {
                        if contracts.len() > 0 {
                            return AddressMsg::Contract(contracts.remove(0));
                        }
                    }

                    AddressMsg::NoContract
                });
                self.status = Some(format!(
                    "Requesting contract for {address} from etherscan.io..."
                ));
                true
            }
            AddressMsg::Contract(contract) => {
                self.status = Some(format!(
                    "Contract for {} found, resolving collection uri...",
                    &contract.contract_name
                ));
                self.name = Some(contract.contract_name);
                self.abi = Some(contract.abi);
                ctx.link().send_message(AddressMsg::ResolveUri);
                true
            }
            AddressMsg::NoContract => {
                self.status = Some(format!("No contract found for {}.", self.address));
                true
            }
            AddressMsg::ResolveUri => {
                let api_key = ctx
                    .props()
                    .api_key
                    .as_ref()
                    .map_or("".to_string(), |k| k.clone());
                let address = self.address;
                let throttle = ctx.props().api_key.as_ref().map_or(THROTTLE_SECONDS, |_| 0);
                if let Some(function) = self.prepare_call() {
                    ctx.link().send_future(async move {
                        {
                            sleep(Duration::from_secs(throttle)).await;
                            Address::call(api_key, address, function).await
                        }
                    });
                }

                false
            }
            AddressMsg::UriResolved(uri_type, uri) => {
                let route = match uri_type {
                    UriType::BaseUri => {
                        debug!(format!("base uri resolved: {uri}"));
                        Route::CollectionToken {
                            // Encode uri
                            uri: crate::uri::Uri::encode(&uri),
                            token: 0,
                        }
                    }
                    UriType::TokenUri => {
                        debug!(format!("token uri resolved: {uri}"));
                        let uri = crate::uri::Uri::parse(&uri, true).unwrap();
                        match uri.token {
                            None => Route::Token { uri: uri.uri },
                            Some(token) => Route::CollectionToken {
                                uri: uri.uri,
                                token,
                            },
                        }
                    }
                };

                ctx.link().history().unwrap().push(route);
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <section class="section is-fullheight">
            if let Some(status) = &self.status {
                <article class="message is-success">
                    <div class="message-body">
                        { status }
                    </div>
                </article>

                if let None = ctx.props().api_key {
                    <article class="message is-danger">
                        <div class="message-body">
                            { format!("Note: No API key has been configured for the etherscan.io API. Requests are \
                            therefore throttled to a single request every {THROTTLE_SECONDS} seconds.") }
                        </div>
                    </article>
                }
            }
            </section>
        }
    }
}

pub enum Msg {
    Navigated,
    TokenStatus(token::Status),
}

pub struct Collection {
    _listener: HistoryListener,
    base_uri: String,
    start_token: usize,
    error: Option<String>,
    requesting_metadata: bool,
    document: Document,
    //token_uri: String,
    token_status_callback: Callback<token::Status>,
    token_status: token::Status,
}

#[derive(PartialEq, Properties)]
pub struct CollectionProps {
    pub uri: String,
    pub token: usize,
}

impl Component for Collection {
    type Message = Msg;
    type Properties = CollectionProps;

    fn create(_ctx: &Context<Self>) -> Self {
        let window = web_sys::window().expect("global window does not exists");
        let document = window.document().expect("expecting a document on window");

        let link = _ctx.link().clone();
        let listener = _ctx.link().history().unwrap().listen(move || {
            link.send_message(Msg::Navigated);
        });

        Self {
            _listener: listener,
            base_uri: _ctx.props().uri.to_string(),
            start_token: 0,
            error: None,
            requesting_metadata: false,
            document,
            //token_uri: format!("{}{}", &_ctx.props().uri, _ctx.props().token),
            token_status_callback: _ctx.link().callback(|status| Msg::TokenStatus(status)),
            token_status: Status::NotStarted,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Navigated => {
                let location = ctx.link().location().unwrap();
                let route = location.route::<Route>().unwrap();
                if let Route::CollectionToken { uri, token } = route {
                    //self.token_uri = format!("{}{token}", uri::Uri::decode(&uri).unwrap());
                }
                true
            }
            Msg::TokenStatus(status) => {
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

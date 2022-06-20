use crate::{cache, models, Route};
use etherscan::contracts::{Contract, ABI};
use etherscan::{Tag, TypeExtensions};
use gloo_timers::future::sleep;
use std::str::FromStr;
use std::time::Duration;
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
                if base_uri.inputs.len() == 0 {
                    if let Ok(encoded) = base_uri.encode_input(&vec![]) {
                        return Some((UriType::BaseUri, hex::encode(&encoded), base_uri.clone()));
                    }
                }
            }
            if let Ok(token_uri) = contract.function("tokenURI") {
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
    ) -> Option<(UriType, String)> {
        let client = etherscan::proxy::Client::new(api_key);

        match client.call(&address, &function.1, Some(Tag::Latest)).await {
            Ok(result) => {
                let decoded = hex::decode(&result[2..]).expect("could not decoded the call result");
                match function.2.decode_output(&decoded) {
                    Ok(tokens) => {
                        return Some((function.0, tokens[0].to_string()));
                    }
                    Err(e) => {
                        log::error!("{:?}", e)
                    }
                }
            }
            Err(e) => {
                log::error!("{:?}", e)
            }
        }
        None
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
    ResolveUri(models::Collection),
    UriResolved(UriType, String, models::Collection),
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
                self.name = Some(contract.contract_name.clone());
                self.abi = Some(contract.abi);
                ctx.link()
                    .send_message(AddressMsg::ResolveUri(models::Collection {
                        name: contract.contract_name,
                        address: Some(TypeExtensions::format(&self.address)),
                        start_token: 0,
                    }));
                true
            }
            AddressMsg::NoContract => {
                self.status = Some(format!("No contract found for {}.", self.address));
                true
            }
            AddressMsg::ResolveUri(collection) => {
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
                            match Address::call(api_key, address, function).await {
                                None => AddressMsg::NoContract,
                                Some((uri_type, uri)) => {
                                    AddressMsg::UriResolved(uri_type, uri, collection)
                                }
                            }
                        }
                    });
                }

                false
            }
            AddressMsg::UriResolved(uri_type, uri, collection) => {
                // Encode uri before passing to route
                let token_uri = crate::uri::TokenUri::parse(&uri, true).unwrap();
                let route = match uri_type {
                    UriType::BaseUri => Route::CollectionToken {
                        uri: token_uri.uri.clone(),
                        token: 0,
                    },
                    UriType::TokenUri => match token_uri.token {
                        None => Route::Token {
                            uri: token_uri.uri.clone(),
                        },
                        Some(token) => Route::CollectionToken {
                            uri: token_uri.uri.clone(),
                            token,
                        },
                    },
                };

                // Cache collection
                if cache::Collection::get(&token_uri.uri).is_none() {
                    cache::Collection::insert(token_uri.uri, collection);
                }

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

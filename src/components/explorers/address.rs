use crate::{cache, Route};
use std::rc::Rc;
use std::str::FromStr;
use workers::etherscan::{Contract, Request, Response, TypeExtensions};
use workers::{Bridge, Bridged};
use yew::prelude::*;
use yew_router::prelude::*;

const THROTTLE_SECONDS: u64 = 5;

pub struct Address {
    worker: Box<dyn Bridge<workers::etherscan::Worker>>,
    status: Option<String>,
}

pub enum AddressMsg {
    CheckAddressType(workers::etherscan::Address),
    Contract(Contract),
    NoContract(workers::etherscan::Address),
    InvalidAddress(String),
    // ResolveUri(models::Collection),
    // UriResolved(UriType, String, models::Collection),
}

#[derive(PartialEq, Properties)]
pub struct AddressProps {
    pub address: String,
    pub api_key: Option<String>,
}

impl Component for Address {
    type Message = AddressMsg;
    type Properties = AddressProps;

    fn create(ctx: &Context<Self>) -> Self {
        // Validate address
        let mut address = None;
        match crate::Address::from_str(&ctx.props().address) {
            Ok(a) => {
                address = Some(a);
                ctx.link().send_message(AddressMsg::CheckAddressType(a));
            }
            Err(_) => {
                ctx.link()
                    .send_message(AddressMsg::InvalidAddress(ctx.props().address.clone()));
            }
        }

        Self {
            worker: workers::etherscan::Worker::bridge(Rc::new({
                let link = ctx.link().clone();
                move |e: workers::etherscan::Response| match e {
                    Response::Contract(contract) => {
                        log::trace!("contract found");
                        link.send_message(Self::Message::Contract(contract))
                    }
                    Response::NoContract(address) => {
                        link.send_message(Self::Message::NoContract(address))
                    }
                    _ => {}
                }
            })),
            status: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            AddressMsg::CheckAddressType(address) => {
                // Check if already resolved to collection
                log::trace!("checking if address already resolved to collection...");
                let key = TypeExtensions::format(&address);
                if let Some(_) = cache::Collection::get(&key) {
                    log::trace!("switching to collection...");
                    // Switch to collection view
                    ctx.link()
                        .history()
                        .unwrap()
                        .push(Route::Collection { id: key });
                    return false;
                }

                // Check if a contract
                log::trace!("checking if address is a contract...");
                self.worker.send(Request::Contract(address));
                self.status = Some(format!(
                    "Checking if address {address} is a contract via etherscan.io..."
                ));
                false
            }
            AddressMsg::Contract(contract) => {
                let address = TypeExtensions::format(&contract.address);
                log::trace!("address {address} is a contract, switching to collection...");
                ctx.link()
                    .history()
                    .unwrap()
                    .push(Route::Collection { id: address });

                // self.status = Some(format!(
                //     "Contract for {} found, resolving collection uri...",
                //     &contract.contract_name
                // ));
                // self.name = Some(contract.contract_name.clone());
                // self.abi = Some(contract.abi);
                // ctx.link()
                //     .send_message(AddressMsg::ResolveUri(models::Collection {
                //         name: contract.contract_name,
                //         address: Some(TypeExtensions::format(&self.address)),
                //         start_token: 0,
                //     }));
                true
            }
            AddressMsg::NoContract(address) => {
                self.status = Some(format!(
                    "No contract found for {address}. Stay tuned for wallet address support",
                ));
                true
            }
            // AddressMsg::ResolveUri(collection) => {
            //     let api_key = ctx
            //         .props()
            //         .api_key
            //         .as_ref()
            //         .map_or("".to_string(), |k| k.clone());
            //     let address = self.address;
            //     let throttle = ctx.props().api_key.as_ref().map_or(THROTTLE_SECONDS, |_| 0);
            //     if let Some(function) = self.prepare_call() {
            //         ctx.link().send_future(async move {
            //             {
            //                 sleep(Duration::from_secs(throttle)).await;
            //                 match Address::call(api_key, address, function).await {
            //                     None => AddressMsg::NoContract,
            //                     Some((uri_type, uri)) => {
            //                         AddressMsg::UriResolved(uri_type, uri, collection)
            //                     }
            //                 }
            //             }
            //         });
            //     }
            //
            //     false
            // }
            // AddressMsg::UriResolved(uri_type, uri, collection) => {
            //     // Encode uri before passing to route
            //     let token_uri = crate::uri::TokenUri::parse(&uri, true).unwrap();
            //     let route = match uri_type {
            //         UriType::BaseUri => Route::CollectionToken {
            //             uri: token_uri.uri.clone(),
            //             token: 0,
            //         },
            //         UriType::TokenUri => match token_uri.token {
            //             None => Route::Token {
            //                 uri: token_uri.uri.clone(),
            //             },
            //             Some(token) => Route::CollectionToken {
            //                 uri: token_uri.uri.clone(),
            //                 token,
            //             },
            //         },
            //     };
            //
            //     // Cache collection
            //     if cache::Collection::get(&token_uri.uri).is_none() {
            //         cache::Collection::insert(token_uri.uri, collection);
            //     }
            //
            //     ctx.link().history().unwrap().push(route);
            //     false
            // }
            AddressMsg::InvalidAddress(address) => {
                self.status = Some(format!("The value of {address} is not a valid address.",));
                true
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

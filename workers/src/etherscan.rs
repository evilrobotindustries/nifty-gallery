use etherscan::{
    contracts::{Contracts, ABI},
    proxy::Proxy,
    APIError,
};
use gloo_timers::future::sleep;
use gloo_worker::{HandlerId, Public, WorkerLink};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::time::Duration;

pub type Address = etherscan::Address;
pub type Function = etherscan::contracts::Function;
pub type TypeExtensions = dyn etherscan::TypeExtensions;
pub type Token = etherscan::contracts::Token;

pub const THROTTLE_SECONDS: u64 = 1;
const RETRY_ATTEMPTS: u8 = 5;

pub struct Worker {
    link: WorkerLink<Self>,
    client: etherscan::Client,
    contracts: HashMap<Address, ABI>,
}

#[derive(Serialize, Deserialize)]
pub enum Request {
    ApiKey(String),
    Contract(Address),
    BaseUri(Address),
    TokenUri(Address, u32),
    TotalSupply(Address),
}

#[derive(Serialize, Deserialize)]
pub enum Response {
    // Contract
    Contract(Contract),
    NoContract(Address),
    ContractFailed(Address, u8),
    // Base URI
    BaseUri(String),
    NoBaseUri(Address),
    BaseUriFailed(Address),
    // Token URI
    TokenUri(String, u32),
    NoTokenUri(Address),
    TokenUriFailed(Address),
    // Total Supply
    TotalSupply(u32),
    NoTotalSupply(Address),
    TotalSupplyFailed(Address),
}

pub enum Message {
    RequestContract(Address, HandlerId),
    Contract(Address, String, ABI, HandlerId),
    NoContract(Address, HandlerId),
    ContractFailed(Address, u8, HandlerId),
    // Base URI
    RequestBaseUri(Address, HandlerId),
    BaseUri(String, HandlerId),
    BaseUriFailed(Address, HandlerId),
    // Token URI
    RequestTokenUri(Address, u32, HandlerId),
    TokenUri(String, u32, HandlerId),
    TokenUriFailed(Address, HandlerId),
    // Total Supply
    RequestTotalSupply(Address, HandlerId),
    TotalSupply(u32, HandlerId),
    TotalSupplyFailed(Address, HandlerId),
}

impl gloo_worker::Worker for Worker {
    type Reach = Public<Self>;
    type Message = Message;
    type Input = Request;
    type Output = Response;

    fn create(link: WorkerLink<Self>) -> Self {
        log::trace!("creating worker...");
        Self {
            link,
            client: etherscan::Client::new(""),
            contracts: HashMap::new(),
        }
    }

    fn update(&mut self, msg: Self::Message) {
        match msg {
            // Contract
            Message::RequestContract(address, id) => {
                log::trace!("requesting contract for {}...", address);
                let client = self.client.clone();
                self.link.send_future(async move {
                    // Call API with retry attempts
                    match Worker::call_api(|| client.get_source_code(&address), RETRY_ATTEMPTS)
                        .await
                    {
                        // Successful
                        Ok(mut contracts) => {
                            if contracts.len() > 0 {
                                let contract = contracts.remove(0);
                                return Message::Contract(
                                    address,
                                    contract.contract_name,
                                    contract.abi,
                                    id,
                                );
                            }

                            Message::NoContract(address, id)
                        }
                        // Failed (after x attempts)
                        Err(e) => Message::ContractFailed(address, RETRY_ATTEMPTS, id),
                    }
                });
            }
            Message::Contract(address, name, abi, id) => {
                log::trace!("contract found at {address}");
                self.contracts.insert(address, abi); // cache abi for subsequent calls
                self.link
                    .respond(id, Response::Contract(Contract { address, name }));
            }
            Message::NoContract(address, id) => {
                log::trace!("no contract for {}...", address);
                self.link.respond(id, Response::NoContract(address));
            }
            Message::ContractFailed(address, attempts, id) => {
                log::error!(
                    "contract at {address} could not be retrieved after {attempts} attempts"
                );
                self.link
                    .respond(id, Response::ContractFailed(address, attempts));
            }
            // Base URI
            Message::RequestBaseUri(address, id) => {
                if let Err(e) = self.call_contract(
                    address,
                    "baseURI",
                    &vec![],
                    id,
                    |tokens, id| Message::BaseUri(tokens[0].to_string(), id),
                    |address, id| Message::BaseUriFailed(address, id),
                ) {
                    match e {
                        ContractError::MissingFunction(_name) => {
                            self.link.respond(id, Response::NoBaseUri(address))
                        }
                        ContractError::MissingContract(address) => {
                            self.update(Message::RequestContract(address, id))
                        }
                        _ => self.link.respond(id, Response::BaseUriFailed(address)),
                    }
                }
            }
            Message::BaseUri(base_uri, id) => {
                log::trace!("base uri succeeded: {base_uri}");
                self.link.respond(id, Response::BaseUri(base_uri));
            }
            Message::BaseUriFailed(address, id) => {
                log::trace!("base uri failed");
                self.link.respond(id, Response::BaseUriFailed(address));
            }
            // Token URI
            Message::RequestTokenUri(address, token, id) => {
                if let Err(e) = self.call_contract(
                    address,
                    "tokenURI",
                    &vec![Token::Uint(token.into())],
                    id,
                    move |tokens, id| Message::TokenUri(tokens[0].to_string(), token, id),
                    move |address, id| Message::TokenUriFailed(address, id),
                ) {
                    match e {
                        ContractError::MissingFunction(_name) => {
                            self.link.respond(id, Response::NoTokenUri(address))
                        }
                        ContractError::MissingContract(address) => {
                            self.update(Message::RequestContract(address, id))
                        }
                        _ => self.link.respond(id, Response::TokenUriFailed(address)),
                    }
                }
            }
            Message::TokenUri(token_uri, token, id) => {
                log::trace!("token uri succeeded: {token_uri}");
                self.link.respond(id, Response::TokenUri(token_uri, token));
            }
            Message::TokenUriFailed(contract, id) => {
                log::trace!("token uri failed");
                self.link.respond(id, Response::TokenUriFailed(contract));
            }
            // Total Supply
            Message::RequestTotalSupply(address, id) => {
                if let Err(e) = self.call_contract(
                    address,
                    "totalSupply",
                    &vec![],
                    id,
                    move |mut tokens, id| match tokens.remove(0).into_uint() {
                        Some(total_supply) => Message::TotalSupply(total_supply.as_u32(), id),
                        None => Message::TotalSupplyFailed(address, id),
                    },
                    move |address, id| Message::TotalSupplyFailed(address, id),
                ) {
                    match e {
                        ContractError::MissingFunction(_name) => {
                            self.link.respond(id, Response::NoTotalSupply(address))
                        }
                        ContractError::MissingContract(address) => {
                            self.update(Message::RequestContract(address, id))
                        }
                        _ => self.link.respond(id, Response::TotalSupplyFailed(address)),
                    }
                }
            }
            Message::TotalSupply(total_supply, id) => {
                log::trace!("total supply succeeded: {total_supply}");
                self.link.respond(id, Response::TotalSupply(total_supply));
            }
            Message::TotalSupplyFailed(address, id) => {
                log::trace!("total supply failed");
                self.link.respond(id, Response::TotalSupplyFailed(address));
            }
        }
    }

    fn handle_input(&mut self, request: Self::Input, id: HandlerId) {
        log::trace!("processing worker request...");
        match request {
            Request::ApiKey(api_key) => self.client.api_key = api_key,
            Request::Contract(address) => self.update(Message::RequestContract(address, id)),
            Request::BaseUri(address) => self.update(Message::RequestBaseUri(address, id)),
            Request::TokenUri(address, token) => {
                self.update(Message::RequestTokenUri(address, token, id))
            }
            Request::TotalSupply(address) => self.update(Message::RequestTotalSupply(address, id)),
        }
    }

    fn name_of_resource() -> &'static str {
        "etherscan.js"
    }
}

impl Worker {
    async fn call_api<C, R, F>(call: C, retry_attempts: u8) -> Result<R, APIError>
    where
        C: Fn() -> F,
        F: Future<Output = Result<R, APIError>>,
    {
        let mut last_error = None;
        for i in 1..retry_attempts {
            match call().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    match e {
                        APIError::RateLimitReached { ref message } => {
                            log::warn!("{message}");
                        }
                        APIError::ContractNotVerified => {
                            log::error!("contract not verified");
                            return Err(e);
                        }
                        APIError::DeserializationError { ref message } => {
                            log::error!("{message}");
                            return Err(e);
                        }
                        APIError::InvalidAddress => {
                            log::error!("invalid address");
                            return Err(e);
                        }
                        APIError::InvalidAPIKey { ref message } => {
                            log::error!("{message}");
                            return Err(e);
                        }
                        APIError::RPCError { code, ref message } => {
                            log::error!("rpc error {code}: {message}");
                        }
                        APIError::TooManyAddresses => {
                            log::error!("too many addresses");
                            return Err(e);
                        }
                        APIError::TransportError { .. } => {
                            log::error!("transport error {e:?}");
                        }
                    }

                    last_error = Some(e);
                    let duration = Duration::from_secs(i.into());
                    log::trace!("retrying in {duration:?}...");
                    sleep(duration).await;
                }
            }
        }
        Err(last_error.unwrap())
    }

    fn call_contract<S, F>(
        &mut self,
        address: Address,
        function: &str,
        inputs: &[Token],
        id: HandlerId,
        success: S,
        fail: F,
    ) -> Result<(), ContractError>
    where
        S: 'static + Fn(Vec<Token>, HandlerId) -> Message,
        F: 'static + Fn(Address, HandlerId) -> Message,
    {
        match self.contracts.get(&address) {
            Some(contract) => {
                match contract.function(function) {
                    Ok(function) => {
                        match function.encode_input(inputs) {
                            Ok(encoded) => {
                                log::trace!(
                                    "calling '{}' function on contract at {address}...",
                                    function.name
                                );
                                let client = self.client.clone();
                                let function = function.clone();
                                let data = hex::encode(&encoded);
                                self.link.send_future(async move {
                                    // Call API with retry attempts
                                    match Worker::call_api(
                                        || {
                                            client.call(
                                                &address,
                                                &data,
                                                Some(etherscan::Tag::Latest),
                                            )
                                        },
                                        RETRY_ATTEMPTS,
                                    )
                                    .await
                                    {
                                        // Successful
                                        Ok(result) => {
                                            // Decode the result
                                            let decoded = hex::decode(&result[2..])
                                                .expect("could not decode the call result");
                                            match function.decode_output(&decoded) {
                                                Ok(tokens) => success(tokens, id),
                                                Err(e) => {
                                                    log::error!("{:?}", e);
                                                    fail(address, id)
                                                }
                                            }
                                        }
                                        // Failed (after x attempts)
                                        Err(e) => fail(address, id),
                                    }
                                });
                                Ok(())
                            }
                            Err(e) => {
                                log::error!("could not encode inputs for '{}' on contract at {address}: {e:?}",
                                    function.name);
                                Err(ContractError::FunctionEncodingError(function.name.clone()))
                            }
                        }
                    }
                    Err(e) => {
                        log::error!(
                            "could not find function '{function}' on contract at {address}: {e:?}"
                        );
                        Err(ContractError::MissingFunction(function.to_string()))
                    }
                }
            }
            None => Err(ContractError::MissingContract(address)),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Contract {
    pub address: Address,
    pub name: String,
}

enum ContractError {
    MissingContract(Address),
    MissingFunction(String),
    FunctionEncodingError(String),
}

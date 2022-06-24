use ethabi::ParamType;
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
    Uri(Address, u32),
    TotalSupply(Address),
}

#[derive(Serialize, Deserialize)]
pub enum Response {
    // Contract
    Contract(Contract),
    NoContract(Address),
    ContractFailed(Address, u8),
    // URI
    Uri(String, Option<u32>),
    NoUri(Address),
    UriFailed(Address),
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
    // URI
    RequestUri(Address, u32, HandlerId),
    Uri(String, Option<u32>, HandlerId),
    UriFailed(Address, HandlerId),
    // Total Supply
    RequestTotalSupply(Address, HandlerId),
    TotalSupply(u32, HandlerId),
    TotalSupplyFailed(Address, HandlerId),
}

const URI_FUNCTIONS: [&str; 3] = ["baseURI", "tokenURI", "uri"];

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
                        Err(_) => Message::ContractFailed(address, RETRY_ATTEMPTS, id),
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
            // URI
            Message::RequestUri(address, token, id) => {
                // Check if contract already exists
                let contract = match self.contracts.get(&address) {
                    None => {
                        log::trace!("contract does not exist locally, requesting...");
                        self.link
                            .send_message(Message::RequestContract(address, id));
                        return;
                    }
                    Some(contract) => contract,
                };

                // Check contract for possible functions
                for name in URI_FUNCTIONS {
                    if let Ok(function) = contract.function(name) {
                        log::trace!(
                            "{name} function found on contract, preparing contract call..."
                        );
                        let mut inputs = Vec::new();
                        match function.inputs.len() {
                            0 => {}
                            1 => {
                                if let ParamType::Uint(_) = function.inputs[0].kind {
                                    inputs.push(Token::Uint(token.into()));
                                }
                            }
                            _ => continue,
                        }

                        // Signal whether url result includes a token
                        let uri_token = if inputs.len() == 1 { Some(token) } else { None };

                        if let Err(_) = self.call_contract(
                            address,
                            function,
                            &inputs,
                            id,
                            move |tokens, id| match tokens.first() {
                                Some(token) => Message::Uri(token.to_string(), uri_token, id),
                                None => {
                                    log::trace!("contract call did not return a result");
                                    Message::UriFailed(address, id)
                                }
                            },
                            move |address, id| Message::UriFailed(address, id),
                        ) {
                            self.link.respond(id, Response::UriFailed(address))
                        }

                        return;
                    }
                }

                self.link.respond(id, Response::NoUri(address));
            }
            Message::Uri(uri, token, id) => {
                log::trace!("uri succeeded: {uri}");
                self.link.respond(id, Response::Uri(uri, token));
            }
            Message::UriFailed(contract, id) => {
                log::trace!("uri failed");
                self.link.respond(id, Response::UriFailed(contract));
            }
            // Total Supply
            Message::RequestTotalSupply(address, id) => {
                // Check if contract already exists
                let contract = match self.contracts.get(&address) {
                    None => {
                        log::trace!("contract does not exist locally, requesting...");
                        self.link
                            .send_message(Message::RequestContract(address, id));
                        return;
                    }
                    Some(contract) => contract,
                };

                // Check for total supply function
                match contract.function("totalSupply") {
                    Err(_) => self.link.respond(id, Response::NoTotalSupply(address)),
                    Ok(function) => {
                        if let Err(_) = self.call_contract(
                            address,
                            function,
                            &vec![],
                            id,
                            move |mut tokens, id| match tokens.remove(0).into_uint() {
                                Some(total_supply) => {
                                    Message::TotalSupply(total_supply.as_u32(), id)
                                }
                                None => Message::TotalSupplyFailed(address, id),
                            },
                            move |address, id| Message::TotalSupplyFailed(address, id),
                        ) {
                            self.link.respond(id, Response::TotalSupplyFailed(address))
                        }
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
            Request::Uri(address, token) => self.update(Message::RequestUri(address, token, id)),
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
        &self,
        address: Address,
        function: &Function,
        inputs: &[Token],
        id: HandlerId,
        success: S,
        fail: F,
    ) -> Result<(), ContractError>
    where
        S: 'static + Fn(Vec<Token>, HandlerId) -> Message,
        F: 'static + Fn(Address, HandlerId) -> Message,
    {
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
                        || client.call(&address, &data, Some(etherscan::Tag::Latest)),
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
                        Err(_) => fail(address, id),
                    }
                });
                Ok(())
            }
            Err(e) => {
                log::error!(
                    "could not encode inputs for '{}' on contract at {address}: {e:?}",
                    function.name
                );
                Err(ContractError::FunctionEncodingError(function.name.clone()))
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Contract {
    pub address: Address,
    pub name: String,
}

enum ContractError {
    FunctionEncodingError(String),
}

use std::rc::Rc;
use workers::etherscan::TypeExtensions;
use workers::{etherscan, Bridge, Bridged};
use yew::prelude::*;
use yew_router::prelude::*;

mod cache;
mod components;
mod models;
mod uri;

extern crate core;

type Address = workers::etherscan::Address;

pub struct App {
    etherscan: Box<dyn Bridge<workers::etherscan::Worker>>,
}

impl Component for App {
    type Message = ();
    type Properties = ();

    fn create(_: &Context<Self>) -> Self {
        if let Err(e) = yew_router_qs::try_route_from_query_string() {
            log::error!("{:?}", e)
        }

        Self {
            // Declare 'globally' so workers not disposed when navigating between components which rely on them
            etherscan: etherscan::Worker::bridge(Rc::new(move |e: etherscan::Response| {})),
        }
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        html! {
            <BrowserRouter>
                <components::Navigation />
                <main>
                    <Switch<Route> render={Switch::render(switch)} />
                </main>
                <components::Footer />
            </BrowserRouter>
        }
    }
}

#[derive(Routable, PartialEq, Clone, Debug)]
pub enum Route {
    #[at("/a/:address")]
    Address { address: String },
    #[at("/c/:id")]
    Collection { id: String },
    #[at("/c/:uri/:token")]
    CollectionToken { uri: String, token: u32 },
    #[at("/")]
    Home,
    #[not_found]
    #[at("/404")]
    NotFound,
    #[at("/t/:uri")]
    Token { uri: String },
}

impl Route {
    fn token(token: &models::Token, collection: Option<&models::Collection>) -> Route {
        match token.id {
            Some(id) => match collection.and_then(|c| c.address) {
                Some(address) => Route::CollectionToken {
                    uri: TypeExtensions::format(&address),
                    token: id,
                },
                None => Route::CollectionToken {
                    uri: uri::encode(&token.url.to_string()),
                    token: id,
                },
            },
            None => Route::Token {
                uri: uri::encode(&token.url.to_string()),
            },
        }
    }
}

fn switch(routes: &Route) -> Html {
    match routes.clone() {
        Route::Address { address } => {
            html! { <components::explorers::address::Address {address} /> }
        }
        Route::Collection { id } => {
            html! { <components::explorers::collection::Collection {id} /> }
        }
        Route::CollectionToken { uri, token } => {
            html! { <components::explorers::collection::CollectionToken {uri} {token} /> }
        }
        Route::Home => {
            html! { <components::Home /> }
        }
        Route::NotFound => {
            html! { <components::NotFound /> }
        }
        Route::Token { uri } => {
            html! {
                <section class="section is-fullheight">
                    <components::token::Token token_uri={uri} />
                </section>
            }
        }
    }
}

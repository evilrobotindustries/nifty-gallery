use serde::{Deserialize, Serialize};
use std::rc::Rc;
use workers::{etherscan, metadata, Bridge, Bridged};
use yew::prelude::*;
use yew_router::prelude::*;

mod components;
mod config;
mod models;
mod notifications;
mod storage;
mod uri;

extern crate core;

type Address = workers::etherscan::Address;

pub struct App {
    _etherscan: Box<dyn Bridge<etherscan::Worker>>,
    _metadata: Box<dyn Bridge<metadata::Worker>>,
}

impl Component for App {
    type Message = ();
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        if let Err(e) = yew_router_qs::try_route_from_query_string() {
            log::error!("{:?}", e)
        }

        Self {
            // Declare workers 'globally' so not disposed when navigating between components which rely on them
            _etherscan: etherscan::Worker::bridge(Rc::new(move |_: etherscan::Response| {})),
            _metadata: metadata::Worker::bridge(Rc::new(move |_: metadata::Response| {})),
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

#[derive(Routable, Eq, Hash, PartialEq, Clone, Debug, Deserialize, Serialize)]
pub enum Route {
    #[at("/a/:address")]
    Address { address: String },
    #[at("/c/:id")]
    Collection { id: String },
    #[at("/c/:id/:token")]
    CollectionToken {
        /// The collection identifier.
        id: String,
        /// The token identifier.
        token: u32,
    },
    #[at("/")]
    Home,
    #[not_found]
    #[at("/404")]
    NotFound,
    // #[at("/t/:uri")]
    // Token { uri: String },
}

impl Route {
    fn token(token: &models::Token, collection: String) -> Route {
        Route::CollectionToken {
            id: collection,
            token: token.id,
        }
    }
}

fn switch(routes: &Route) -> Html {
    match routes.clone() {
        Route::Address { address } => {
            html! { <components::address::Address { address } /> }
        }
        Route::Collection { id } => {
            html! { <components::collection::Collection { id } /> }
        }
        Route::CollectionToken { id, token } => {
            html! { <components::collection::token::Token collection={ id } { token } /> }
        }
        Route::Home => {
            html! { <components::Home /> }
        }
        Route::NotFound => {
            html! { <components::NotFound /> }
        } // Route::Token { uri } => {
          //     html! {
          //         <section class="section is-fullheight">
          //             <components::token::Token token_uri={uri} />
          //         </section>
          //     }
          // }
    }
}

pub struct Scroll {}

impl Scroll {
    fn top(window: &web_sys::Window) {
        let mut scroll_options = web_sys::ScrollToOptions::new();
        scroll_options.top(0.0);
        scroll_options.behavior(web_sys::ScrollBehavior::Smooth);
        window.scroll_to_with_scroll_to_options(&scroll_options);
    }
}

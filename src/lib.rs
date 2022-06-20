use yew::prelude::*;
use yew_router::prelude::*;

mod cache;
mod components;
mod models;
mod uri;

extern crate core;

type Address = etherscan::Address;

#[derive(Routable, PartialEq, Clone, Debug)]
pub enum Route {
    #[at("/a/:address")]
    Address { address: String },
    // #[at("/c/:uri")]
    // Collection,
    #[at("/c/:uri/:token")]
    CollectionToken { uri: String, token: usize },
    #[at("/")]
    Home,
    #[not_found]
    #[at("/404")]
    NotFound,
    #[at("/t/:uri")]
    Token { uri: String },
}

pub struct App {}

impl Component for App {
    type Message = ();
    type Properties = ();

    fn create(_: &Context<Self>) -> Self {
        if let Err(e) = yew_router_qs::try_route_from_query_string() {
            log::error!("{:?}", e)
        }

        Self {}
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

fn switch(routes: &Route) -> Html {
    match routes.clone() {
        Route::Address { address } => {
            html! { <components::explorers::address::Address {address} /> }
        }
        // Route::Collection => {
        //     html! { <components::explorers::Collection /> }
        // }
        Route::CollectionToken { uri, token } => {
            html! { <components::explorers::collection::Collection {uri} {token} /> }
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

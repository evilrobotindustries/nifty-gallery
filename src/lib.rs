extern crate core;

use yew::prelude::*;
use yew_router::prelude::*;

pub mod agents;
mod cache;
mod components;
mod metadata;
mod models;
mod uri;

type Address = etherscan::Address;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

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

    fn create(_ctx: &Context<Self>) -> Self {
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
                    <Switch<Route> render={switch} />
                </main>
                <footer class="footer">
                    <div class="content has-text-centered">
                        <p>{"© 2022 Nifty Gallery"}</p>
                        <p>{"Powered by "}<a href="https://etherscan.io">{"Etherscan.io"}</a>{" APIs"}</p>
                        <p>
                            { "Site by " }<a href="https://evilrobot.industries" target="_blank">{ "Evil Robot \
                            Industries" }</a>
                        </p>
                    </div>
                </footer>
            </BrowserRouter>
        }
    }
}

fn switch(routes: Route) -> Html {
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

fn main() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Trace));
    yew::Renderer::<App>::new().render();
}

use once_cell::sync::Lazy;
use std::rc::Rc;
use workers::etherscan::TypeExtensions;
use workers::{etherscan, metadata, Bridge, Bridged};
use yew::prelude::*;
use yew_router::prelude::*;

mod cache;
mod components;
mod models;
mod uri;

extern crate core;

type Address = workers::etherscan::Address;

pub struct App {
    etherscan: Box<dyn Bridge<etherscan::Worker>>,
    metadata: Box<dyn Bridge<metadata::Worker>>,
}

impl Component for App {
    type Message = ();
    type Properties = ();

    fn create(_: &Context<Self>) -> Self {
        if let Err(e) = yew_router_qs::try_route_from_query_string() {
            log::error!("{:?}", e)
        }

        Self {
            // Declare workers 'globally' so not disposed when navigating between components which rely on them
            etherscan: etherscan::Worker::bridge(Rc::new(move |e: etherscan::Response| {})),
            metadata: metadata::Worker::bridge(Rc::new(move |e: metadata::Response| {})),
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
    fn collection(id: &str, collection: &models::Collection) -> Route {
        match collection.address {
            Some(address) => Route::Collection {
                id: TypeExtensions::format(&address),
            },
            None => Route::CollectionToken {
                uri: id.to_string(),
                token: collection.start_token,
            },
        }
    }

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

static COLLECTIONS: Lazy<Vec<(&str, &str, &str, u32)>> = Lazy::new(|| {
    vec![
        (
            "Azuki",
            "0xed5af388653567af2f388e6224dc7c4b3241c544",
            "https://ikzttp.mypinata.cloud/ipfs/QmQFkLSQysj94s5GvTHPyzTxrawwtjgiiYS2TBLgrvw8CW/",
            10_000,
        ),
        (
            "Bored Ape Yacht Club",
            "0xbc4ca0eda7647a8ab7c2061c2e118a18a936f13d",
            "https://ipfs.io/ipfs/QmeSjSinHpPnmXmspMjwiXyN6zS4E9zccariGR3jxcaWtq/",
            10_000,
        ),
        (
            "Clone X",
            "0x49cf6f5d44e70224e2e23fdcdd2c053f30ada28b",
            "https://clonex-assets.rtfkt.com/",
            19_311,
        ),
        (
            "Cool Cats NFT",
            "0x1a92f7381b9f03921564a437210bb9396471050c",
            "https://api.coolcatsnft.com/cat/",
            9_941,
        ),
        (
            "Doodles",
            "0x8a90cab2b38dba80c64b7734e58ee1db38b8992e",
            "https://ipfs.io/ipfs/QmPMc4tcBsMqLRuCQtPmPe84bpSjrC3Ky7t3JWuHXYB4aS/",
            10_000,
        ),
        (
            "Meebits",
            "0x7bd29408f11d2bfc23c34f18275bbf23bb716bc7",
            "https://meebits.larvalabs.com/meebit/",
            20_000,
        ),
        (
            "Moonbirds",
            "0x23581767a106ae21c074b2276d25e5c3e136a68b",
            "https://live---metadata-5covpqijaa-uc.a.run.app/metadata/",
            10_000,
        ),
        (
            "Mutant Ape Yacht Club",
            "0x60e4d786628fea6478f785a6d7e704777c86a7c6",
            "https://boredapeyachtclub.com/api/mutants/",
            19_423,
        ),
        (
            "Otherdeed for Otherside",
            "0x34d85c9cdeb23fa97cb08333b511ac86e1c4e258",
            "https://api.otherside.xyz/lands/",
            100_000,
        ),
    ]
});

use crate::components::token::RecentTokens;
use crate::models::Collection;
use crate::storage::All;
use crate::{models, storage, uri, Address, Route, Scroll};
use itertools::Itertools;
use once_cell::sync::Lazy;
use std::ops::Deref;
use std::str::FromStr;
use wasm_bindgen::JsCast;
use web_sys::{HtmlElement, HtmlInputElement, Node};
use workers::etherscan::TypeExtensions;
use yew::prelude::*;
use yew_router::prelude::*;

pub mod address;
pub mod collection;
pub mod token;

#[function_component(Footer)]
pub fn footer() -> yew::Html {
    html! {
        <footer class="footer">
            <div class="content has-text-centered">
            <p>{"© 2022 Nifty Gallery"}</p>
            <p>{"Powered by "}<a href="https://etherscan.io">{"Etherscan.io"}</a>{" APIs"}</p>
            <p>{ "Site by " }<a href="https://evilrobot.industries" target="_blank">
                { "Evil Robot Industries" }</a>
            </p>
            </div>
        </footer>
    }
}

#[function_component(Home)]
pub fn home() -> yew::Html {
    html! {
        <section class="hero is-fullheight">
            <div class="hero-body">
                <div class="container has-text-centered">
                    <div class="column is-6 is-offset-3">
                        <p class="subtitle">
                            { "Nifty Gallery, a tool for exploring NFT collections." }
                        </p>
                        <Search />
                    </div>
                    <section class="section" style="overflow:hidden">
                        <RecentTokens />
                    </section>
                </div>
            </div>
        </section>
    }
}

fn collections() -> Vec<Html> {
    let mut collections: Vec<Html> = Vec::new();

    fn html<'a>(collections: impl Iterator<Item = &'a models::Collection>) -> Vec<Html> {
        collections
            .filter_map(|c| {
                c.name().map(|name| {
                    let route = Route::Collection { id: c.id() };
                    html! {
                        <Link<Route> to={route}>
                            <div class="dropdown-item">{ name }</div>
                        </Link<Route>>
                    }
                })
            })
            .collect()
    }

    // Add recent collections
    let mut recent = html(
        storage::Collection::get()
            .iter()
            .filter(|collection| collection.last_viewed().is_some())
            .sorted_by_key(|collection| collection.last_viewed().unwrap())
            .rev(),
    );
    if recent.len() > 0 {
        // Add header
        collections.push(html! {
            <div class="dropdown-header dropdown-item">
                { "Recent Collections" }
            </div>
        });
    }
    collections.append(&mut recent);

    if collections.len() > 0 {
        collections.push(html! { <hr class="dropdown-divider" /> });
    }

    // Add top collections
    collections.push(html! {
        <div class="dropdown-header dropdown-item">
            { "Notable Collections" }
        </div>
    });
    collections.append(&mut html(
        TOP_COLLECTIONS
            .iter()
            .sorted_by_key(|collection| collection.name().unwrap().clone()),
    ));

    collections
}

static TOP_COLLECTIONS: Lazy<Vec<models::Collection>> = Lazy::new(|| {
    let collections = crate::config::COLLECTIONS
        .iter()
        .map(|(name, address, base_uri, total_supply)| {
            Collection::new(address, name, base_uri, *total_supply)
        })
        .collect();

    // Add items to local storage
    for collection in &collections {
        if !storage::Collection::contains(collection) {
            storage::Collection::store(collection.clone());
        }
    }

    collections
});

#[function_component(Navigation)]
pub fn nav() -> yew::Html {
    use_effect(move || {
        let window = web_sys::window().expect("global window does not exists");
        let document = window.document().expect("expecting a document on window");
        // Add navigation listeners
        bulma::add_navigation_listeners(&document);
        || ()
    });

    // Scroll to top of page on navigation
    if let Some(history) = use_history() {
        use_state(|| {
            history.listen(|| {
                if let Some(window) = web_sys::window() {
                    Scroll::top(&window);
                }
            })
        });
    }

    html! {
        <nav class="navbar is-fixed-top" role="navigation" aria-label="main navigation">
            <div class="navbar-brand">
                <Link<Route> classes={classes!("navbar-item")} to={Route::Home}>
                    { "NIFTY GALLERY" }
                </Link<Route>>

                // <a href="javascript:void(0);" role="button" class="navbar-burger" aria-label="menu"
                //     aria-expanded="false" data-target="navbarBasicExample">
                //   <span aria-hidden="true"></span>
                //   <span aria-hidden="true"></span>
                //   <span aria-hidden="true"></span>
                // </a>
            </div>

            // <div class="navbar-menu">
            //     <div class="navbar-start"></div>
            //
            //     <div class="navbar-end">
            //         <div class="navbar-item navbar-descriptor">{"Explore by: "}</div>
            //         <Link<Route> classes={classes!("navbar-item")} to={Route::Address} disabled=true>
            //             { "Address" }
            //         </Link<Route>>
            //         <Link<Route> classes={classes!("navbar-item")} to={Route::Collection}>
            //             { "Collection" }
            //         </Link<Route>>
            //     </div>
            // </div>
        </nav>
    }
}

#[function_component(NotFound)]
pub fn not_found() -> yew::Html {
    html! {
        <section class="hero is-danger is-bold is-large">
            <div class="hero-body">
                <div class="container">
                    <h1 class="title">
                        { "Page not found" }
                    </h1>
                    <h2 class="subtitle">
                        { "Page page does not seem to exist" }
                    </h2>
                </div>
            </div>
        </section>
    }
}

#[function_component(Search)]
pub fn search() -> yew::Html {
    let history = use_history().unwrap();
    let input_change = Callback::from(move |e: Event| {
        let input: HtmlInputElement = e.target_unchecked_into();
        let value = input.value();

        // Check for address
        if let Ok(address) = Address::from_str(&value) {
            history.clone().push(Route::Address {
                address: TypeExtensions::format(&address),
            })
        } else if let Ok(uri) = uri::TokenUri::parse(&value, true) {
            if let Some(token) = uri.token {
                history.clone().push(Route::CollectionToken {
                    id: uri.to_string().into(),
                    token,
                })
            } else {
                todo!()
                // history.clone().push(Route::Token {
                //     uri: uri.to_string().into(),
                // })
            }
        } else {
            todo!()
        }
    });
    let on_focus_in = Callback::from(move |e: FocusEvent| {
        e.target_unchecked_into::<HtmlElement>()
            .closest(".dropdown")
            .ok()
            .and_then(|e| e)
            .map(|e| e.class_list().add_1("is-active"));
    });
    let on_focus_out = Callback::from(move |e: FocusEvent| {
        let dropdown = e
            .target_unchecked_into::<HtmlElement>()
            .closest(".dropdown")
            .ok()
            .and_then(|e| e)
            .expect("could not find dropdown");
        // Ignore if related target
        if let Some(event_target) = e.related_target() {
            if let Some(node) = event_target.dyn_ref::<Node>() {
                if dropdown.contains(Some(node)) {
                    return;
                }
            }
        }
        let _ = dropdown.class_list().remove_1("is-active");
    });
    html! {
        <div id="search" class="field is-horizontal">
            <div class="field-body">
                <div class="field has-addons dropdown">
                    <div class="control has-icons-left is-expanded"
                         onfocusin={ on_focus_in }
                         onfocusout={ on_focus_out }
                         aria-haspopup="true"
                         aria-controls="dropdown-menu">
                        <input class="input"
                               type="text"
                               placeholder="Enter contract address or token metadata URL"
                               onchange={ input_change } />
                        <span class="icon is-small is-left">
                            <i class="fas fa-globe"></i>
                        </span>
                    </div>
                    <div class="control">
                        <a href="javascript:void(0);" class="button is-primary">
                            {"Explore"}
                        </a>
                    </div>

                    <div class="dropdown-menu" id="dropdown-menu" role="menu">
                        <div class="dropdown-content">
                            { collections() }
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}

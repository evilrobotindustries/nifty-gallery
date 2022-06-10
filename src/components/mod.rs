pub mod explorers;
pub mod token;

use crate::components::token::RecentTokens;
use crate::{cache, uri, Address, Route};
use itertools::Itertools;
use std::str::FromStr;
use wasm_bindgen::JsCast;
use web_sys::{HtmlElement, HtmlInputElement, Node};
use yew::prelude::*;
use yew_router::prelude::*;

#[function_component(Home)]
pub fn home() -> yew::Html {
    let navigator = use_navigator().unwrap();
    let input_change = Callback::from(move |e: Event| {
        let input: HtmlInputElement = e.target_unchecked_into();
        let value = input.value();

        // Check for address
        if let Ok(address) = Address::from_str(&value) {
            navigator.clone().push(&Route::Address {
                address: etherscan::TypeExtensions::format(&address),
            })
        } else if let Ok(uri) = uri::TokenUri::parse(&value, true) {
            if let Some(token) = uri.token {
                navigator.clone().push(&Route::CollectionToken {
                    uri: uri.to_string().into(),
                    token,
                })
            } else {
                navigator.clone().push(&Route::Token {
                    uri: uri.to_string().into(),
                })
            }
        } else {
            todo!()
        }
    });
    let on_focus_in = Callback::from(move |e: FocusEvent| {
        let input: HtmlElement = e.target_unchecked_into();
        input
            .offset_parent()
            .expect("could not find element parent")
            .class_list()
            .add_1("is-active");
    });
    let on_focus_out = Callback::from(move |e: FocusEvent| {
        let input: HtmlElement = e.target_unchecked_into();
        let parent = input
            .offset_parent()
            .expect("could not find element parent");
        // Ignore if related target
        if let Some(event_target) = e.related_target() {
            if let Some(node) = event_target.dyn_ref::<Node>() {
                if parent.contains(Some(node)) {
                    return;
                }
            }
        }
        parent.class_list().remove_1("is-active");
    });
    let collections: Option<Vec<Html>> = cache::Collection::items().map_or(None, |collections| {
        Some(
            collections
                .into_iter()
                .sorted_by_key(|(_, collection)| collection.name.clone())
                .map(|collection| {
                    let route = Route::CollectionToken {
                        uri: collection.0,
                        token: collection.1.start_token as usize,
                    };
                    html! {
                        <Link<Route> to={route}>
                            <div class="dropdown-item">
                                { collection.1.name }
                            </div>
                        </Link<Route>>
                    }
                })
                .collect(),
        )
    });
    html! {
        <section class="hero is-fullheight">
            <div class="hero-body">
                <div class="container has-text-centered">
                    <div class="column is-6 is-offset-3">
                        <p class="subtitle">
                            { "Nifty Gallery, a tool for exploring NFT collections." }
                        </p>
                        <div class="field is-horizontal">
                            <div class="field-body">
                                <div class="field has-addons">
                                    <div class="control has-icons-left is-expanded dropdown"
                                         onfocusin={ on_focus_in }
                                         onfocusout={ on_focus_out }
                                         aria-haspopup="true"
                                         aria-controls="dropdown-menu">
                                        <input class="input"
                                               type="text"
                                               placeholder="Enter token URL or contract address"
                                               onchange={ input_change } />
                                        <span class="icon is-small is-left">
                                            <i class="fas fa-globe"></i>
                                        </span>
                                        if let Some(collections) = collections {
                                            <div class="dropdown-menu" id="dropdown-menu" role="menu">
                                                <div class="dropdown-content">
                                                    { collections }
                                                </div>
                                            </div>
                                        }
                                    </div>
                                    <div class="control">
                                        <a href="javascript:void(0);" class="button is-primary">
                                            {"Explore"}
                                        </a>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                    <section class="section" style="overflow:hidden">
                        <RecentTokens />
                    </section>
                </div>
            </div>
        </section>
    }
}

#[function_component(Navigation)]
pub fn nav() -> yew::Html {
    use_effect(move || {
        let window = web_sys::window().expect("global window does not exists");
        let document = window.document().expect("expecting a document on window");
        // Add navigation listeners
        bulma::add_navigation_listeners(&document);
        || ()
    });

    html! {
        <nav class="navbar" role="navigation" aria-label="main navigation">
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

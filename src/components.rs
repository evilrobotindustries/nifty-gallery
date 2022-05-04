use crate::Route;
use yew::prelude::*;
use yew_router::prelude::Link;

#[function_component(Home)]
pub fn home() -> yew::Html {
    html! {}
}

#[function_component(Navigation)]
pub fn nav() -> yew::Html {
    html! {
        <nav class="navbar" role="navigation" aria-label="main navigation">
            <div class="navbar-brand">
                <Link<Route> classes={classes!("navbar-item")} to={Route::Home}>
                    { "NIFTY GALLERY" }
                </Link<Route>>

                <a role="button" class="navbar-burger" aria-label="menu" aria-expanded="false" data-target="navbarBasicExample">
                  <span aria-hidden="true"></span>
                  <span aria-hidden="true"></span>
                  <span aria-hidden="true"></span>
                </a>
            </div>

            <div id="navbarBasicExample" class="navbar-menu">
                <div class="navbar-start"></div>

                <div class="navbar-end">
                    <Link<Route> classes={classes!("navbar-item")} to={Route::Explorer}>
                        { "Explorer" }
                    </Link<Route>>
                </div>
            </div>
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

pub mod explorer {
    use gloo_console::{debug, error};
    use gloo_net::http::Request;
    use itertools::Itertools;
    use std::collections::HashMap;
    use std::str::FromStr;
    use web_sys::{HtmlInputElement, Url};
    use yew::prelude::*;

    pub enum Msg {
        UriChanged(String),
        UriFailed(String),
        RequestMetadata,
        MetadataLoaded(crate::metadata::Metadata),
        Previous,
        Next,
    }

    pub struct Model {
        base_uri: Option<String>,
        token: usize,
        error: Option<String>,
        metadata: Option<crate::metadata::Metadata>,
    }

    impl Component for Model {
        type Message = Msg;
        type Properties = ();

        fn create(_ctx: &Context<Self>) -> Self {
            if let Err(e) = yew_router_qs::try_route_from_query_string() {
                error!(e)
            }

            Self {
                base_uri: None,
                token: 0,
                error: None,
                metadata: None,
            }
        }

        fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
            match msg {
                Msg::UriChanged(uri) => {
                    self.error = None;

                    if uri == "" {
                        return false;
                    }

                    // parse uri
                    match Url::new(&uri) {
                        Ok(url) => {
                            // Get token from path
                            let path = url.pathname();
                            let segments: Vec<&str> = path.split('/').collect();

                            match segments.last() {
                                Some(token) => {
                                    // Set base uri and token
                                    self.base_uri =
                                        Some(uri[..uri.len() - token.len()].to_string());
                                    match usize::from_str(token) {
                                        Ok(token) => {
                                            self.token = token;
                                            _ctx.link().send_message(Msg::RequestMetadata);
                                            false
                                        }
                                        Err(e) => {
                                            self.error = Some(format!(
                                                "could not parse the token id: {:?}",
                                                e
                                            ));
                                            true
                                        }
                                    }
                                }
                                None => {
                                    self.error = Some(format!("could not parse the token id"));
                                    true
                                }
                            }
                        }
                        Err(e) => {
                            self.error = Some(format!("could not parse the url: {:?}", e));
                            true
                        }
                    }
                }
                Msg::MetadataLoaded(metadata) => {
                    debug!(format!("{:?}", metadata));
                    self.metadata = Some(metadata);
                    true
                }
                Msg::UriFailed(error) => {
                    self.error = Some(error);
                    true
                }
                Msg::RequestMetadata => {
                    let uri = format!("{}{}", self.base_uri.as_ref().unwrap(), self.token);
                    _ctx.link().send_future(async move {
                        match Request::get(&uri).send().await {
                            Ok(response) => {
                                if response.status() == 200 {
                                    // debug!(format!("{:?}", response));
                                    // let s = response.text().await.unwrap();
                                    // debug!(format!("{}", s));
                                    match response.json::<crate::metadata::Metadata>().await {
                                        Ok(metadata) => Msg::MetadataLoaded(metadata),
                                        Err(e) => Msg::UriFailed(format!("{e}")),
                                    }
                                    //Msg::UriFailed(format!("{s}"))
                                } else {
                                    Msg::UriFailed(format!(
                                        "Request failed: {} {}",
                                        response.status(),
                                        response.status_text()
                                    ))
                                }
                            }
                            Err(e) => Msg::UriFailed(format!("{e}")),
                        }
                    });

                    false
                }
                Msg::Previous => {
                    self.token -= 1;
                    _ctx.link().send_message(Msg::RequestMetadata);
                    false
                }
                Msg::Next => {
                    self.token += 1;
                    _ctx.link().send_message(Msg::RequestMetadata);
                    false
                }
            }
        }

        fn view(&self, ctx: &Context<Self>) -> Html {
            let uri = self
                .base_uri
                .as_ref()
                .map_or("".to_string(), |u| format!("{u}{}", self.token));
            let uri_change = ctx.link().callback(move |e: Event| {
                let input: HtmlInputElement = e.target_unchecked_into();
                Msg::UriChanged(input.value())
            });
            let previous_click = ctx.link().callback(|_| Msg::Previous);
            let next_click = ctx.link().callback(|_| Msg::Next);
            let error = self
                .error
                .as_ref()
                .map_or("".to_string(), |u| u.to_string());
            html! {
                    <div class="section">
                        <div class="field">
                            <label class="label">{"URL"}</label>
                            <div class="control">
                                <input class="input" type="text" placeholder="Enter token URL" value={ uri }
                                    onchange={ uri_change } />
                            </div>
                        </div>

                        if self.error.is_some() {
                            <div class="notification is-danger">
                              { error }
                            </div>
                        }

                        <div class="field is-grouped">
                          <div class="control">
                            <button class="button is-primary" onclick={ previous_click } disabled={ self.token == 0 }>{"Previous"}</button>
                          </div>
                          <div class="control">
                            <button class="button is-primary" onclick={ next_click } >{"Next"}</button>
                          </div>
                        </div>

                        if self.metadata.is_some() {
                            <Metadata name={ self.metadata.as_ref().unwrap().name.clone() }
                                                  description={ self.metadata.as_ref().unwrap().description.clone() }
                                                  attributes={ map(self.metadata.as_ref().unwrap()) }
                                                  image={ self.metadata.as_ref().unwrap().image.clone() } />
                        }
                    </div>
            }
        }
    }

    pub fn map(metadata: &crate::metadata::Metadata) -> HashMap<String, String> {
        metadata.attributes.iter().map(|a| a.map()).collect()
    }

    #[derive(Properties, PartialEq)]
    pub struct MetadataProps {
        pub name: String,
        pub description: String,
        pub image: String,
        pub attributes: HashMap<String, String>,
        pub external_url: Option<String>,
    }

    #[function_component(Metadata)]
    pub fn metadata(props: &MetadataProps) -> yew::Html {
        let attributes: Html = props
            .attributes
            .iter()
            .sorted_by_key(|a| a.0)
            .map(|a| {
                html! {
                    <div class="control">
                        <div class="tags has-addons">
                            <span class="tag">{a.0}</span>
                            <span class="tag">{a.1}</span>
                        </div>
                    </div>
                }
            })
            .collect();
        html! {
            <>
                <h1 class="title">{ &props.name }</h1>
                <div class="content">{ &props.description }</div>
                <div class="field is-grouped is-grouped-multiline">{ attributes }</div>
                <img src={ props.image.clone() } />
            </>
        }
    }
}

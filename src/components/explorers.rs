use crate::components::token;
use crate::components::token::{Status, Token};
use crate::{uri, Route};
use web_sys::Document;
use yew::prelude::*;
use yew_router::prelude::*;

#[function_component(Address)]
pub fn address() -> yew::Html {
    html! {}
}

pub enum Msg {
    Navigated,
    TokenStatus(token::Status),
}

#[derive(PartialEq, Properties)]
pub struct Props {
    pub uri: String,
    pub token: usize,
}

pub struct Collection {
    _listener: HistoryListener,
    base_uri: String,
    start_token: usize,
    error: Option<String>,
    requesting_metadata: bool,
    document: Document,
    token_uri: String,
    token_status_callback: Callback<token::Status>,
    token_status: token::Status,
}

impl Component for Collection {
    type Message = Msg;
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        let window = web_sys::window().expect("global window does not exists");
        let document = window.document().expect("expecting a document on window");

        let link = _ctx.link().clone();
        let listener = _ctx.link().history().unwrap().listen(move || {
            link.send_message(Msg::Navigated);
        });

        Self {
            _listener: listener,
            base_uri: _ctx.props().uri.to_string(),
            start_token: 0,
            error: None,
            requesting_metadata: false,
            document,
            token_uri: format!("{}{}", &_ctx.props().uri, _ctx.props().token),
            token_status_callback: _ctx.link().callback(|status| Msg::TokenStatus(status)),
            token_status: Status::NotStarted,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Navigated => {
                let location = ctx.link().location().unwrap();
                let route = location.route::<Route>().unwrap();
                if let Route::CollectionToken { uri, token } = route {
                    self.token_uri = format!("{}{token}", uri::Uri::decode(&uri).unwrap());
                }

                true
            }
            Msg::TokenStatus(status) => {
                self.token_status = status;
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let base_uri_encoded = uri::Uri::encode(&self.base_uri);
        let status = self.token_status_callback.clone();
        let token = ctx.props().token;

        html! {
            <section class="section is-fullheight">
                if let Some(error) = &self.error {
                    <div class="notification is-danger">
                      { error }
                    </div>
                }

                // todo: raise up to navigation to optimise space
                <div class="level is-mobile">
                    <div class="level-left"></div>
                    <div class="level-right">
                        <div class="field has-addons">
                          <div class="control">
                            if token > 0 {
                                <Link<Route> classes="button is-primary" to={Route::CollectionToken {
                                    uri: base_uri_encoded.clone(), token: token - 1 }}
                                    disabled={ self.requesting_metadata || token == self.start_token }>
                                    <span class="icon is-small">
                                      <i class="fas fa-angle-left"></i>
                                    </span>
                                </Link<Route>>
                            }
                          </div>
                          <div class="control">
                            <Link<Route> classes="button is-primary" to={Route::CollectionToken {
                                uri: base_uri_encoded.clone(), token: token + 1 }}
                                disabled={ self.requesting_metadata }>
                                <span class="icon is-small">
                                  <i class="fas fa-angle-right"></i>
                                </span>
                            </Link<Route>>
                          </div>
                        </div>
                    </div>
                </div>

                <Token uri={ self.token_uri.clone() } token={ ctx.props().token } {status} />

                if matches!(self.token_status, Status::NotFound) && ctx.props().token != self.start_token {
                    <article class="message is-primary">
                        <div class="message-body">
                            {"The requested token was not found. Have you reached the end of the collection? Click "}
                            <Link<Route> to={Route::CollectionToken {
                                uri: base_uri_encoded, token: self.start_token }}>
                                {"here"}
                            </Link<Route>>
                            {" to return to the start of the collection."}
                        </div>
                    </article>
                }
            </section>
        }
    }

    fn rendered(&mut self, _ctx: &Context<Self>, _first_render: bool) {
        // Wire up full screen image modal
        bulma::add_modals(&self.document);
    }
}

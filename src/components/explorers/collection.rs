use crate::components::token;
use crate::components::token::{Status, Token};
use crate::{cache, Route};
use web_sys::Document;
use yew::prelude::*;
use yew_router::prelude::*;

pub enum Msg {
    Navigated,
    TokenStatus(token::Status),
}

pub struct Collection {
    //_listener: HistoryListener,
    base_uri: String,
    start_token: usize,
    error: Option<String>,
    requesting_metadata: bool,
    document: Document,
    token_status_callback: Callback<token::Status>,
    token_status: token::Status,
}

#[derive(PartialEq, Properties)]
pub struct CollectionProps {
    pub uri: String,
    pub token: usize,
}

impl Component for Collection {
    type Message = Msg;
    type Properties = CollectionProps;

    fn create(_ctx: &Context<Self>) -> Self {
        let window = web_sys::window().expect("global window does not exists");
        let document = window.document().expect("expecting a document on window");

        //let link = _ctx.link().clone();
        // let listener = _ctx.link().history().unwrap().listen(move || {
        //     link.send_message(Msg::Navigated);
        // });

        Self {
            //_listener: listener,
            base_uri: _ctx.props().uri.to_string(),
            start_token: 0,
            error: None,
            requesting_metadata: false,
            document,
            //token_uri: format!("{}{}", &_ctx.props().uri, _ctx.props().token),
            token_status_callback: _ctx.link().callback(|status| Msg::TokenStatus(status)),
            token_status: Status::NotStarted,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Navigated => {
                // let location = ctx.link().location().unwrap();
                // let route = location.route::<Route>().unwrap();
                // if let Route::CollectionToken { uri, token } = route {
                //     //self.token_uri = format!("{}{token}", uri::Uri::decode(&uri).unwrap());
                // }
                true
            }
            Msg::TokenStatus(status) => {
                if matches!(status, Status::NotFound) && ctx.props().token == 0 {
                    let uri = &ctx.props().uri;
                    let start_token = ctx.props().token + 1;
                    if let Some(mut collection) = cache::Collection::get(&uri) {
                        collection.start_token = start_token as u8;
                        cache::Collection::insert(uri.clone(), collection);
                    }
                    ctx.link()
                        .navigator()
                        .unwrap()
                        .push(&Route::CollectionToken {
                            uri: uri.clone(),
                            token: start_token,
                        });
                    return false;
                }

                self.token_status = status;
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
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
                                    uri: self.base_uri.clone(), token: token - 1 }}
                                    disabled={ self.requesting_metadata || token == self.start_token }>
                                    <span class="icon is-small">
                                      <i class="fas fa-angle-left"></i>
                                    </span>
                                </Link<Route>>
                            }
                          </div>
                          <div class="control">
                            <Link<Route> classes="button is-primary" to={Route::CollectionToken {
                                uri: self.base_uri.clone(), token: token + 1 }}
                                disabled={ self.requesting_metadata }>
                                <span class="icon is-small">
                                  <i class="fas fa-angle-right"></i>
                                </span>
                            </Link<Route>>
                          </div>
                        </div>
                    </div>
                </div>

                <Token token_uri={ self.base_uri.clone() } token_id={ ctx.props().token } {status} />

                if matches!(self.token_status, Status::NotFound) && ctx.props().token != self.start_token {
                    <article class="message is-primary">
                        <div class="message-body">
                            {"The requested token was not found. Have you reached the end of the collection? Click "}
                            <Link<Route> to={Route::CollectionToken {
                                uri: self.base_uri.clone(), token: self.start_token }}>
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

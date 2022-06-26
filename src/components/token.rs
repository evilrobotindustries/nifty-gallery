use crate::storage::{Get, RecentlyViewedItem};
use crate::{models, storage, uri, Route};
use bulma::carousel::Options;
use itertools::Itertools;
use std::rc::Rc;
use workers::metadata::Response;
use workers::{metadata, qr, Bridge, Bridged};
use yew::prelude::*;
use yew_router::prelude::*;

pub struct Token {
    qr: Box<dyn Bridge<qr::Worker>>,
    /// The qr code of the current url
    qr_code: Option<String>,
}

#[derive(Debug)]
pub enum Message {
    // Qr Code
    GenerateQRCode,
    QRCode(String),
}

#[derive(Properties)]
pub struct Properties {
    pub token: Rc<models::Token>,
}

impl PartialEq for Properties {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.token, &other.token)
    }
}

impl Component for Token {
    type Message = Message;
    type Properties = Properties;

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_message(Message::GenerateQRCode);

        Self {
            qr: qr::Worker::bridge(Rc::new({
                let link = ctx.link().clone();
                move |e: qr::Response| link.send_message(Self::Message::QRCode(e.qr_code))
            })),
            qr_code: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Message::GenerateQRCode => {
                if let Some(location) = web_sys::window()
                    .and_then(|window| window.document())
                    .and_then(|document| document.location())
                    .and_then(|location| location.href().ok())
                {
                    log::trace!("generating qr code...");
                    self.qr.send(workers::qr::Request { url: location });
                }
                false
            }
            Message::QRCode(qr_code) => {
                log::trace!("qr code generated");
                self.qr_code = Some(qr_code);
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let props = ctx.props();
        html! {
            if let Some(metadata) = props.token.metadata.as_ref() {
                <div class="card columns">
                if let Some((video, poster)) = props.video() {
                    <div class="column">
                        <figure class="image">
                            <video class="modal-button" data-target="nifty-image" controls={true}
                                    poster={ poster.clone() }>
                                <source src={ video.clone() } type="video/mp4" />
                            </video>
                        </figure>
                        <div id="nifty-image" class="modal modal-fx-3dFlipHorizontal">
                            <div class="modal-background"></div>
                            <div class="modal-content">
                                <p class="image">
                                    <video class="modal-button" data-target="nifty-image" controls={true}
                                            poster={ poster }>
                                        <source src={ video } type="video/mp4" />
                                    </video>
                                </p>
                            </div>
                            <button class="modal-close is-large" aria-label="close"></button>
                        </div>
                    </div>
                }
                else {
                    <div class="column">
                        <figure class="image">
                            <img src={ metadata.image.clone() } alt={ metadata.name.clone() } class="modal-button"
                                 data-target="nifty-image" />
                        </figure>
                        <div id="nifty-image" class="modal modal-fx-3dFlipHorizontal">
                            <div class="modal-background"></div>
                            <div class="modal-content">
                                <p class="image">
                                    <img src={ metadata.image.clone() } alt={ metadata.name.clone() } />
                                </p>
                            </div>
                            <button class="modal-close is-large" aria-label="close"></button>
                        </div>
                    </div>
                }
                    <div class="column">
                        <div class="card-content">
                            <h1 class="title nifty-name">{ props.name() }</h1>
                            <div class="content">{ props.description() }</div>
                            <div class="field is-grouped is-grouped-multiline">{ props.attributes() }</div>
                            if let Some(external_url) = &metadata.external_url {
                                <div class="content">
                                    <a href={ external_url.to_string() } target="_blank">
                                        <i class="fa-solid fa-globe"></i>
                                    </a>
                                </div>
                            }
                            <table class="table">
                            <tbody>
                            if let Some(last_viewed) = &props.token.last_viewed {
                                <tr>
                                    <th>{"Last viewed: "}</th>
                                    <td>{ last_viewed }</td>
                                </tr>
                            }
                            </tbody>
                            </table>
                        </div>
                        <footer class="card-footer">
                            <div class="card-content level is-mobile">
                                <div class="level-left">
                                    <div class="level-item has-text-centered">
                                        <div>
                                            <p class="heading">{"Traits"}</p>
                                            <p class="title">{ props.total_attributes() }</p>
                                        </div>
                                    </div>
                                </div>
                                <div class="level-right">
                                    if let Some(qr_code) = self.qr_code.as_ref() {
                                        <figure class="image is-qr-code level-item">
                                            <img src={ qr_code.clone() } alt={ metadata.name.clone() } />
                                        </figure>
                                    }
                                </div>
                            </div>
                        </footer>
                    </div>
                </div>
            }
        }
    }

    fn rendered(&mut self, _ctx: &Context<Self>, _first_render: bool) {
        if let Some(document) = web_sys::window().and_then(|window| window.document()) {
            // Wire up full screen image modal
            bulma::add_modals(&document);
        }
    }
}

impl Properties {
    fn attributes(&self) -> Html {
        self.token
            .metadata
            .as_ref()
            .map_or(Html::default(), |metadata| {
                let attributes: Vec<(String, String)> =
                    metadata.attributes.iter().map(|a| a.map()).collect();

                attributes
                    .iter()
                    .sorted_by_key(|a| &a.0)
                    .map(|a| {
                        html! {
                            <div class="control">
                                <div class="tags has-addons">
                                    <span class="tag">{ &a.0 }</span>
                                    <span class="tag">{ &a.1 }</span>
                                </div>
                            </div>
                        }
                    })
                    .collect()
            })
    }

    fn total_attributes(&self) -> usize {
        self.token.metadata.as_ref().map_or(0, |metadata| {
            metadata
                .attributes
                .iter()
                .map(|a| a.map())
                .filter(|a| a.1 != "None")
                .count()
        })
    }

    fn description(&self) -> &str {
        self.token.metadata.as_ref().map_or("", |metadata| {
            metadata
                .description
                .as_ref()
                .map_or("", |description| description)
        })
    }

    fn name(&self) -> String {
        self.token
            .metadata
            .as_ref()
            .map_or("".to_string(), |metadata| {
                metadata
                    .name
                    .as_ref()
                    .map_or(self.token.id.to_string(), |name| name.to_string())
            })
    }

    fn video(&self) -> Option<(String, String)> {
        self.token
            .metadata
            .as_ref()
            .map_or(None, |metadata| match &metadata.animation_url {
                None => None,
                Some(animation_url) => Some((animation_url.clone(), metadata.image.clone())),
            })
    }
}

#[function_component(RecentTokens)]
pub fn recent_tokens() -> yew::Html {
    use_effect(move || {
        // Attach carousel after component is rendered
        bulma::carousel::attach(Some("#recent-views"), Some(Options { slides_to_show: 4 }));
        || {}
    });
    let slides: Option<Vec<Html>> = storage::RecentlyViewed::values().map_or(None, |recent| {
        Some(
            recent
                .into_iter()
                .rev()
                .map(|item| {
                    html! {
                        <Link<Route> to={ item.route }>
                            <figure class="image">
                                <img src={ item.image } alt={ item.name } />
                            </figure>
                        </Link<Route>>
                    }
                })
                .collect(),
        )
    });
    html! {
        if let Some(slides) = slides {
            <p class="subtitle">{"Recently Viewed"}</p>
            <div id="recent-views">
                { slides }
            </div>
        }
    }
}

#[derive(Debug)]
pub enum Status {
    NotStarted,
    Requesting,
    Completed,
    NotFound,
    Failed,
}

//
// pub struct Token {
//     metadata: Box<dyn Bridge<metadata::Worker>>,
//     qr: Box<dyn Bridge<qr::Worker>>,
//     // The current token
//     token: Option<crate::models::Token>,
//
//     // If applicable, the corresponding collection
//     collection: Option<crate::models::Collection>,
//     // Any error, if applicable
//     error: Option<String>,
//     /// The qr code of the current url
//     qr_code: Option<String>,
//
//     requesting: Option<crate::models::Token>,
// }
//
// impl Token {
//     fn attributes(&self) -> Html {
//         self.token.as_ref().map_or(Html::default(), |token| {
//             token.metadata().map_or(Html::default(), |metadata| {
//                 let attributes: Vec<(String, String)> =
//                     metadata.attributes.iter().map(|a| a.map()).collect();
//
//                 attributes
//                     .iter()
//                     .sorted_by_key(|a| &a.0)
//                     .map(|a| {
//                         html! {
//                             <div class="control">
//                                 <div class="tags has-addons">
//                                     <span class="tag">{ &a.0 }</span>
//                                     <span class="tag">{ &a.1 }</span>
//                                 </div>
//                             </div>
//                         }
//                     })
//                     .collect()
//             })
//         })
//     }
//
//     fn description(&self) -> &str {
//         self.token.as_ref().map_or("", |token| {
//             token.metadata().map_or("", |metadata| {
//                 metadata
//                     .description
//                     .as_ref()
//                     .map_or("", |description| description)
//             })
//         })
//     }
//
//     fn name(&self) -> String {
//         self.token.as_ref().map_or("".to_string(), |token| {
//             token.metadata().map_or("".to_string(), |metadata| {
//                 metadata.name.as_ref().map_or(
//                     // Use token number for missing name (if available)
//                     match &self.collection {
//                         None => token.id.map_or("".to_string(), |token| token.to_string()),
//                         Some(collection) => token.id.map_or("".to_string(), |token| {
//                             format!("{} {}", collection.name().unwrap_or(""), token)
//                         }),
//                     },
//                     |name| name.to_string(),
//                 )
//             })
//         })
//     }
//
//     fn traits(&self) -> usize {
//         self.token.as_ref().map_or(0, |token| {
//             token.metadata.as_ref().map_or(0, |metadata| {
//                 metadata
//                     .attributes
//                     .iter()
//                     .map(|a| a.map())
//                     .filter(|a| a.1 != "None")
//                     .count()
//             })
//         })
//     }
//
//     fn video(&self) -> Option<(String, String)> {
//         self.token.as_ref().map_or(None, |token| {
//             token
//                 .metadata
//                 .as_ref()
//                 .map_or(None, |metadata| match &metadata.animation_url {
//                     None => None,
//                     Some(animation_url) => Some((animation_url.clone(), metadata.image.clone())),
//                 })
//         })
//     }
// }
//
// #[derive(Debug)]
// pub enum Message {
//     Request(String),
//     GenerateQRCode,
//     Redirect(String, crate::models::Token),
//     Completed(crate::models::Token),
//     Metadata(workers::metadata::Metadata),
//     QRCode(String),
//     Failed(String),
//     NotFound,
// }
//
// #[derive(PartialEq, Properties)]
// pub struct Properties {
//     pub token: models::Token,
//     #[prop_or_default]
//     pub status: Callback<Status>,
// }

// impl Component for Token {
//     type Message = Message;
//     type Properties = Props;
//
//     fn create(ctx: &Context<Self>) -> Self {
//         ctx.link().send_message(Message::GenerateQRCode);
//         ctx.link()
//             .send_message(Message::Request(ctx.props().metadata_url));
//
//         Self {
//             metadata: metadata::Worker::bridge(Rc::new({
//                 let link = ctx.link().clone();
//                 move |e: metadata::Response| match e {
//                     Response::Completed(metadata, _) => {
//                         link.send_message(Self::Message::Metadata(metadata))
//                     }
//                     Response::NotFound(_, _) => {}
//                     Response::Failed(_, _) => {}
//                 }
//             })),
//             qr: qr::Worker::bridge(Rc::new({
//                 let link = ctx.link().clone();
//                 move |e: qr::Response| link.send_message(Self::Message::QRCode(e.qr_code))
//             })),
//             token: None,
//             collection: storage::Collection::get(ctx.props().token_uri.as_str()),
//             error: None,
//             qr_code: None,
//
//             requesting: None,
//         }
//     }
//
//     fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
//         match msg {
//             Message::Request(collection, token) => {
//                 self.error = None;
//
//                 // Check local storage
//                 log::trace!("checking local storage");
//                 if let Some(token) = storage::Token::get(collection.as_str(), token) {
//                     ctx.link().send_message(Message::Completed(token));
//                     return true;
//                 }
//
//                 log::trace!("requesting metadata from worker");
//                 self.metadata.send(workers::metadata::Request {
//                     url: token.url.to_string(),
//                     token: token.id,
//                     cors_proxy: Some(crate::config::CORS_PROXY.to_string()),
//                 });
//                 self.requesting = Some(token);
//                 // ctx.link()
//                 //     .send_future(async move { token::request_metadata(url, token).await });
//
//                 ctx.props().status.emit(Status::Requesting);
//
//                 true
//             }
//             Message::Redirect(url, token) => {
//                 self.metadata.send(workers::metadata::Request {
//                     url,
//                     token: token.id,
//                     cors_proxy: Some(crate::config::CORS_PROXY.to_string()),
//                 });
//                 self.error = None;
//                 ctx.props().status.emit(Status::Requesting);
//                 true
//             }
//             Message::Completed(mut token) => {
//                 // Update token
//                 if let Some(mut metadata) = token.metadata.as_mut() {
//                     if let Ok(token_uri) = uri::TokenUri::parse(&metadata.image, false) {
//                         metadata.image = match token_uri.token {
//                             None => token_uri.uri,
//                             Some(id) => format!("{}{}", token_uri.uri, id),
//                         };
//
//                         storage::RecentlyViewed::insert(RecentlyViewedItem {
//                             name: metadata
//                                 .name
//                                 .as_ref()
//                                 .map_or_else(|| "".to_string(), |n| n.clone()),
//                             image: metadata.image.clone(),
//                             route: Route::CollectionToken {
//                                 id: ctx.props().token_uri.clone(),
//                                 token: ctx.props().token_id.expect("expected a token identifier"),
//                             },
//                         })
//                     }
//                 }
//                 ctx.props().status.emit(Status::Completed);
//                 self.token = Some(token.clone());
//
//                 // Store token
//                 log::trace!("adding to local storage");
//                 token.last_viewed = Some(chrono::offset::Utc::now());
//                 storage::Token::insert(token.url.as_str(), token.clone());
//                 log::trace!("cached");
//                 true
//             }
//             Message::Failed(error) => {
//                 ctx.props().status.emit(Status::Failed);
//                 self.error = Some(error);
//                 true
//             }
//             Message::NotFound => {
//                 self.token = None;
//                 ctx.props().status.emit(Status::NotFound);
//                 true
//             }
//             Message::Metadata(metadata) => {
//                 if let Some(mut token) = self.requesting.take() {
//                     log::trace!("response completed {:?}", metadata);
//                     token.metadata = Some(metadata);
//                     ctx.link().send_message(Message::Completed(token));
//                 }
//                 //ctx.link().send_message(Msg::Completed(metadata))
//                 false
//             }
//         }
//     }
//
//     fn changed(&mut self, ctx: &Context<Self>) -> bool {
//         let uri = uri::decode(&ctx.props().token_uri).expect("unable to decode the uri");
//         let id = ctx.props().token_id;
//         if self.token.is_none()
//             || uri != self.token.as_ref().unwrap().url.to_string()
//             || id != self.token.as_ref().unwrap().id
//         {
//             log::trace!("token changed, requesting metadata...");
//             ctx.link().send_message(Message::GenerateQRCode);
//             ctx.link().send_message(Message::Request(
//                 crate::models::Token::create(uri, id).expect("unable to create token"),
//             ));
//         }
//         false
//     }
//
//     fn view(&self, _ctx: &Context<Self>) -> Html {
//         html! {
//             <>
//                 if let Some(error) = &self.error {
//                     <div class="notification is-danger">
//                       { error }
//                     </div>
//                 }
//
//                 if let Some(token) = self.token.as_ref() {
//
//                 }
//             </>
//         }
//     }
// }

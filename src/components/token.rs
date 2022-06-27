use crate::{models, storage, Route};
use bulma::carousel::Options;
use itertools::Itertools;
use std::rc::Rc;
use workers::{qr, Bridge, Bridged};
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

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
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

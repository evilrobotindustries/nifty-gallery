use gloo_console::error;
use yew::prelude::*;
use yew_router::prelude::*;

mod components;
mod metadata;

#[derive(Routable, PartialEq, Clone, Debug)]
pub enum Route {
    #[at("/address")]
    Address,
    #[at("/collection")]
    Collection,
    #[at("/")]
    Home,
    #[not_found]
    #[at("/404")]
    NotFound,
}

struct Model {}

impl Component for Model {
    type Message = ();
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        if let Err(e) = yew_router_qs::try_route_from_query_string() {
            error!(e)
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
                <footer class="footer">
                    <div class="content has-text-centered">
                        { "Powered by " }
                        <a href="https://evilrobot.industries" target="_blank">{ "Evil Robot Industries" }</a>
                    </div>
                </footer>
            </BrowserRouter>
        }
    }
}

fn switch(routes: &Route) -> Html {
    match routes.clone() {
        Route::Address => {
            html! { <components::explorers::Address /> }
        }
        Route::Collection => {
            html! { <components::explorers::Collection /> }
        }
        Route::Home => {
            html! { <components::Home /> }
        }
        Route::NotFound => {
            html! { <components::NotFound /> }
        }
    }
}

fn main() {
    yew::start_app::<Model>();
}

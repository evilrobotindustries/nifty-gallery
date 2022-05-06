use gloo_console::error;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, KeyboardEvent, NodeList};

pub fn add_modals(document: &Document) {
    // Add a click event on buttons to open a specific modal
    if let Ok(modal_buttons) = document.query_selector_all(".modal-button") {
        for button in modal_buttons.to_list::<Element>() {
            let target = button
                .get_attribute("data-target")
                .expect("could not find data-target attribute on modal button");
            let target = document
                .get_element_by_id(&target)
                .expect("could not find target element");

            // Add event listener
            let target_clone = target.clone();
            let listener = Closure::wrap(Box::new(move |_error: JsValue| {
                if let Err(e) = target_clone.class_list().add_1("is-active") {
                    error!("unable to add is-active class to modal: {:?}", e)
                }
            }) as Box<dyn Fn(JsValue)>);
            if let Err(e) =
                button.add_event_listener_with_callback("click", listener.as_ref().unchecked_ref())
            {
                error!(
                    "unable to add click event listener to modal button: {:?}",
                    e
                )
            }
            listener.forget();
        }
    }

    // Add a click event on various child elements to close the parent modal
    if let Ok(modal_buttons) = document.query_selector_all(
        ".modal-background, .modal-close, .modal-card-head .delete, .modal-card-foot .button",
    ) {
        for close in modal_buttons.to_list::<Element>() {
            if let Some(target) = close
                .closest(".modal")
                .expect("could not find closest modal")
            {
                // Add event listener
                let target_clone = target.clone();
                let listener = Closure::wrap(Box::new(move |_error: JsValue| {
                    if let Err(e) = target_clone.class_list().remove_1("is-active") {
                        error!("unable to remove is-active class from modal: {:?}", e)
                    }
                }) as Box<dyn Fn(JsValue)>);
                if let Err(e) = close
                    .add_event_listener_with_callback("click", listener.as_ref().unchecked_ref())
                {
                    error!(
                        "unable to add click event listener to close button: {:?}",
                        e
                    )
                }
                listener.forget();
            }
        }
    }

    // Add a keyboard event to close all modals
    let document_clone = document.clone();
    let listener = Closure::wrap(Box::new(move |e: KeyboardEvent| {
        // Check for escape key
        if e.key_code() == 27 {
            if let Ok(modals) = document_clone.query_selector_all(".modal") {
                for modal in modals.to_list::<Element>() {
                    if let Err(e) = modal.class_list().remove_1("is-active") {
                        error!("unable to remove is-active class from modal: {:?}", e)
                    }
                }
            }
        }
    }) as Box<dyn Fn(KeyboardEvent)>);
    if let Err(e) =
        document.add_event_listener_with_callback("keydown", listener.as_ref().unchecked_ref())
    {
        error!("unable to add keydown event listener to document: {:?}", e)
    }
    listener.forget();
}

pub fn add_navigation_listeners(document: &Document) {
    // Check if there are any navbar burgers
    if let Ok(burgers) = document.query_selector_all(".navbar-burger") {
        let nav = document
            .get_elements_by_tag_name("nav")
            .item(0)
            .expect("could not find nav element");

        // Add a click event on each of them
        for burger in burgers.to_list::<Element>() {
            // Get the target from the "data-target" attribute
            let target = burger
                .get_attribute("data-target")
                .expect("could not find data-target attribute on burger");
            let target = document
                .get_element_by_id(&target)
                .expect("could not find target element");

            // Add click event listener to burger
            let nav = nav.clone();
            let burger_clone = burger.clone();
            let target_clone = target.clone();
            let listener = Closure::wrap(Box::new(move |_error: JsValue| {
                // Toggle the "is-active" class on the "navbar-burger" and the "navbar-menu"
                if let Err(e) = nav.class_list().toggle("is-active") {
                    error!(format!("unable to toggle is-active for nav: {:?}", e))
                }
                if let Err(e) = burger_clone.class_list().toggle("is-active") {
                    error!(format!("unable to toggle is-active for burger: {:?}", e))
                }
                if let Err(e) = target_clone.class_list().toggle("is-active") {
                    error!(format!(
                        "unable to toggle is-active for burger target: {:?}",
                        e
                    ))
                }
            }) as Box<dyn Fn(JsValue)>);
            if let Err(e) =
                burger.add_event_listener_with_callback("click", listener.as_ref().unchecked_ref())
            {
                error!("unable to add click event listener to burger: {:?}", e)
            }
            listener.forget();

            // Add listener to navbar items to close menu when clicked
            match target.query_selector_all(".navbar-item") {
                Ok(navbar_items) => {
                    let navbar_items = navbar_items.to_list::<Element>();
                    for item in &navbar_items {
                        let item_clone = item.clone();
                        let burger_clone = burger.clone();
                        let target_clone = target.clone();
                        let navbar_items = navbar_items.clone();
                        // Close menu when item clicked
                        let listener = Closure::wrap(Box::new(move |_error: JsValue| {
                            for item in &navbar_items {
                                if let Err(e) = item.class_list().remove_1("is-active") {
                                    error!("unable to remove is-active from navbar-item: {:?}", e)
                                }
                            }

                            if let Err(e) = item_clone.class_list().toggle("is-active") {
                                error!(format!(
                                    "unable to toggle is-active for navbar item: {:?}",
                                    e
                                ))
                            }
                            if let Err(e) = burger_clone.class_list().toggle("is-active") {
                                error!(format!("unable to toggle is-active for burger: {:?}", e))
                            }
                            if let Err(e) = target_clone.class_list().toggle("is-active") {
                                error!(format!("unable to toggle is-active for target: {:?}", e))
                            }
                        })
                            as Box<dyn Fn(JsValue)>);
                        if let Err(e) = item.add_event_listener_with_callback(
                            "click",
                            listener.as_ref().unchecked_ref(),
                        ) {
                            error!("unable to add click event listener to navbar item: {:?}", e)
                        }
                        listener.forget();
                    }
                }
                Err(error) => error!(error),
            }
        }
    }
}

pub trait ElementList {
    fn to_list<T: AsRef<Element> + JsCast>(self) -> Vec<T>;
}

impl ElementList for NodeList {
    fn to_list<T>(self) -> Vec<T>
    where
        T: AsRef<Element> + JsCast,
    {
        let mut result = Vec::with_capacity(self.length() as usize);

        for index in 0..self.length() {
            if let Some(item) = self.get(index) {
                if !item.has_type::<T>() {
                    continue;
                }
                let item = item
                    .dyn_into::<T>()
                    .expect("could not cast node to element");
                result.push(item);
            }
        }

        result
    }
}

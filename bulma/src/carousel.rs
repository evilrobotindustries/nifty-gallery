use serde::Serialize;
use wasm_bindgen::prelude::*;

pub fn attach(selector: Option<&str>, options: Option<Options>) {
    default::attach(
        selector,
        options.map_or(JsValue::null(), |o| {
            JsValue::from_serde(&o).expect("could not serialise options")
        }),
    );
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Options {
    pub slides_to_show: u8,
}

#[wasm_bindgen(module = "/assets/bulma-carousel.min.js")]
extern "C" {
    #[allow(non_camel_case_types)]
    type default;

    #[wasm_bindgen(static_method_of = default)]
    pub fn attach(selector: Option<&str>, options: JsValue);
}

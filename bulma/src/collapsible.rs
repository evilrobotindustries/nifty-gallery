use wasm_bindgen::prelude::*;

pub fn attach() {
    default::attach();
}

#[wasm_bindgen(module = "/assets/bulma-collapsible.min.js")]
extern "C" {
    #[allow(non_camel_case_types)]
    type default;

    #[wasm_bindgen(static_method_of = default)]
    pub fn attach();
}

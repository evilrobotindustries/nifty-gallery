pub use gloo_worker::{Bridge, Bridged, PublicWorker};
pub use url::{ParseError, Url};

pub mod etherscan;
pub mod metadata;
pub mod qr;

// Workaround to enable fetch api for worker: https://github.com/rustwasm/gloo/issues/201#issuecomment-1078454938
mod fetch {

    use gloo_net::Error;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    pub(crate) async fn get(url: &str) -> Result<Response, Error> {
        let mut opts = web_sys::RequestInit::new();
        opts.method("GET");
        let request = web_sys::Request::new_with_str_and_init(url, &opts).map_err(js_to_error)?;

        let global = js_sys::global();
        let worker = global
            .dyn_into::<web_sys::DedicatedWorkerGlobalScope>()
            .unwrap();

        let promise = worker.fetch_with_request(&request);

        let response = JsFuture::from(promise).await.map_err(js_to_error)?;
        match response.dyn_into::<web_sys::Response>() {
            Ok(response) => Ok(Response(response)),
            Err(e) => panic!("fetch returned {:?}, not `Response` - this is a bug", e),
        }
    }

    fn js_to_error(js_value: wasm_bindgen::JsValue) -> Error {
        Error::JsError(js_to_js_error(js_value))
    }

    fn js_to_js_error(js_value: wasm_bindgen::JsValue) -> gloo_utils::errors::JsError {
        match gloo_utils::errors::JsError::try_from(js_value) {
            Ok(error) => error,
            Err(_) => unreachable!("JsValue passed is not an Error type -- this is a bug"),
        }
    }

    pub(crate) struct Response(web_sys::Response);

    impl Response {
        pub fn headers(&self) -> gloo_net::http::Headers {
            gloo_net::http::Headers::from_raw(self.0.headers())
        }

        pub fn status(&self) -> u16 {
            self.0.status()
        }
        pub fn status_text(&self) -> String {
            self.0.status_text()
        }

        pub async fn text(&self) -> Result<String, Error> {
            let promise = self.0.text().unwrap();
            let val = JsFuture::from(promise).await.map_err(js_to_error)?;
            let string = js_sys::JsString::from(val);
            Ok(String::from(&string))
        }
    }
}

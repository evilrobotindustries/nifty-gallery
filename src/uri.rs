use base64::DecodeError;
use std::str;
use std::str::FromStr;
use url::{ParseError, Url};

pub struct Uri {
    pub base_uri: String,
    pub token: usize,
}

impl Uri {
    pub fn parse(input: &str) -> Result<Uri, ParseError> {
        let mut url = Url::parse(input)?;
        if url.scheme() == "ipfs" {
            // Convert IPFS protocol address to IPFS gateway
            // ( preserve existing object to preserve additional attributes like query string parameters etc.)
            let cid = url
                .host_str()
                .expect("could not get host name from url")
                .to_string();
            url.set_host(Some("ipfs.io"))?;
            url.set_path(&format!("/ipfs/{}{}", cid, url.path()));

            // New instance required due to internal url rules about changing schemes
            url = Url::parse(&url.to_string().replace("ipfs://", "https://"))
                .expect("could not parse url converted from ipfs to https")
        }

        // Get token from path
        let segments: Vec<&str> = url
            .path_segments()
            .expect("could not get path segments from url")
            .collect();

        let token = segments.last().unwrap();
        let uri = url.to_string();
        let base_uri = uri[..uri.len() - token.len()].to_string();
        let token = usize::from_str(token).unwrap_or(0);

        Ok(Uri {
            base_uri: Uri::encode(&base_uri),
            token,
        })
    }

    pub fn decode(input: &str) -> Result<String, DecodeError> {
        Ok(
            str::from_utf8(&base64::decode_config(input, base64::URL_SAFE_NO_PAD)?)
                .expect("could not decode utf8 string")
                .to_string(),
        )
    }

    pub fn encode(input: &str) -> String {
        base64::encode_config(input, base64::URL_SAFE_NO_PAD)
    }
}

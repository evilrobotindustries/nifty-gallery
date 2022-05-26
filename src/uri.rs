use base64::DecodeError;
use std::str;
use std::str::FromStr;
use url::{ParseError, Url};

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

#[derive(Debug)]
pub struct TokenUri {
    pub uri: String,
    pub token: Option<usize>,
    pub encoded: bool,
}

impl TokenUri {
    pub fn parse(input: &str, encode: bool) -> Result<TokenUri, ParseError> {
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

        let mut uri = url.to_string();
        let mut token = None;
        if let Some(segment) = segments.last() {
            if let Ok(t) = usize::from_str(segment) {
                uri = uri[..uri.len() - segment.len()].to_string();
                token = Some(t);
            }
        }

        if encode {
            uri = crate::uri::encode(&uri)
        }
        Ok(TokenUri {
            uri,
            token,
            encoded: encode,
        })
    }

    pub fn to_string(&self) -> &str {
        &self.uri
    }
}

#[cfg(test)]
mod tests {
    use crate::uri::TokenUri;

    #[test]
    fn parses_base_uri() {
        let uri = "https://api.site.com/token/";
        let encoded = crate::uri::encode(uri);
        let uri = TokenUri::parse(uri, true).expect("could not parse uri");
        assert_eq!(encoded, uri.uri);
        assert_eq!(None, uri.token);
        assert!(uri.encoded);
    }

    #[test]
    fn parses_ipfs_base_uri() {
        let uri = "https://ipfs.io/ipfs/QmeSjSinHpPnmXmspMjwiXyN6zS4E9zccariGR3jxcaWtq/";
        let encoded = crate::uri::encode(uri);
        let uri = TokenUri::parse(
            "ipfs://QmeSjSinHpPnmXmspMjwiXyN6zS4E9zccariGR3jxcaWtq/",
            true,
        )
        .expect("could not parse uri");
        assert_eq!(encoded, uri.uri);
        assert_eq!(None, uri.token);
        assert!(uri.encoded);
    }
}

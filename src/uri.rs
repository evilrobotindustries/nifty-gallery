use base64::DecodeError;
use std::str;
use std::str::FromStr;
use workers::{ParseError, Url};

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

pub fn parse(input: &str) -> Result<Url, ParseError> {
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
    Ok(url)
}

#[derive(Debug)]
pub struct TokenUri {
    pub uri: String,
    pub token: Option<u32>,
    pub encoded: bool,
}

impl TokenUri {
    pub fn parse(input: &str, encode: bool) -> Result<TokenUri, ParseError> {
        // Get token from path
        let url = parse(input)?;
        let segments: Vec<&str> = url
            .path_segments()
            .expect("could not get path segments from url")
            .collect();

        let mut uri = url.to_string();
        let mut token = None;
        if let Some(segment) = segments.last() {
            if let Ok(t) = u32::from_str(segment) {
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
    use crate::uri::parse;

    #[test]
    fn parses_base_uri() {
        let uri = "https://api.site.com/token/";
        let url = parse(uri).expect("could not parse uri");
        assert_eq!(uri, url.as_str());
    }

    #[test]
    fn parses_ipfs_base_uri() {
        let uri = "https://ipfs.io/ipfs/QmeSjSinHpPnmXmspMjwiXyN6zS4E9zccariGR3jxcaWtq/";
        let url = parse("ipfs://QmeSjSinHpPnmXmspMjwiXyN6zS4E9zccariGR3jxcaWtq/")
            .expect("could not parse uri");
        assert_eq!(uri, url.as_str());
    }
}

use serde::de::{self};
use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;
use std::marker::PhantomData;

#[derive(Deserialize, Debug)]
pub struct Metadata {
    // Name of the item.
    pub name: String,
    // A human readable description of the item. Markdown is supported.
    pub description: String,
    /// This is the URL to the image of the item. Can be just about any type of image (including SVGs, which will be cached into PNGs by OpenSea), and can be IPFS URLs or paths. We recommend using a 350 x 350 image.
    pub image: String,
    // This is the URL that will appear below the asset's image on OpenSea and will allow users to leave OpenSea and view the item on your site.
    pub external_url: Option<String>,
    // These are the attributes for the item, which will show up on the OpenSea page for the item. (see below)
    #[serde(deserialize_with = "sequence_or_map")]
    pub attributes: Vec<Attribute>,
    // Background color of the item on OpenSea. Must be a six-character hexadecimal without a pre-pended #.
    pub background_color: Option<String>,
    // A URL to a multi-media attachment for the item. The file extensions GLTF, GLB, WEBM, MP4, M4V, OGV, and OGG are supported, along with the audio-only extensions MP3, WAV, and OGA.
    // Animation_url also supports HTML pages, allowing you to build rich experiences and interactive NFTs using JavaScript canvas, WebGL, and more. Scripts and relative paths within the HTML page are now supported. However, access to browser extensions is not supported.
    pub animation_url: Option<String>,
    // A URL to a YouTube video.
    pub youtube_url: Option<String>,
}

#[derive(Deserialize, Debug)]
pub enum Attribute {
    String {
        trait_type: String,
        value: String,
    },
    // Numeric
    Number {
        trait_type: String,
        value: usize,
        max_value: Option<usize>,
    },
    BoostPercentage {
        trait_type: String,
        value: f32,
        max_value: Option<usize>,
    },
    BoostNumber {
        trait_type: String,
        value: f32,
        max_value: Option<usize>,
    },
    // Date
    Date {
        trait_type: String,
        // A unix timestamp (seconds)
        value: usize,
    },
}

impl Attribute {
    pub fn map(&self) -> (String, String) {
        match self {
            Attribute::String { trait_type, value } => (trait_type.to_string(), value.to_string()),
            Attribute::Number {
                trait_type, value, ..
            } => (trait_type.to_string(), value.to_string()),
            Attribute::BoostPercentage {
                trait_type, value, ..
            } => (trait_type.to_string(), value.to_string()),
            Attribute::BoostNumber {
                trait_type, value, ..
            } => (trait_type.to_string(), value.to_string()),
            Attribute::Date { trait_type, value } => (trait_type.to_string(), value.to_string()),
        }
    }
}

fn sequence_or_map<'de, D>(deserializer: D) -> Result<Vec<Attribute>, D::Error>
where
    D: Deserializer<'de>,
{
    // This is a Visitor that forwards string types to T's `FromStr` impl and
    // forwards map types to T's `Deserialize` impl. The `PhantomData` is to
    // keep the compiler from complaining about T being an unused generic type
    // parameter. We need T in order to know the Value type for the Visitor
    // impl.
    struct SequenceOrMap<T>(PhantomData<fn() -> T>);

    impl<'de> Visitor<'de> for SequenceOrMap<Vec<Attribute>> {
        type Value = Vec<Attribute>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("sequence or map")
        }

        fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            Deserialize::deserialize(de::value::SeqAccessDeserializer::new(seq))
        }

        fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Vec<Attribute>, M::Error> {
            let mut attributes = Vec::<Attribute>::new();
            while let Some(key) = map.next_key()? {
                attributes.push(Attribute::String {
                    trait_type: key,
                    value: map.next_value()?,
                })
            }
            Ok(attributes)
        }
    }

    deserializer.deserialize_any(SequenceOrMap(PhantomData))
}

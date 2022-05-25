use chrono::{DateTime, Utc};
use gloo_console::debug;
use serde::de::{self};
use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::marker::PhantomData;
use std::str::FromStr;
use std::{f32, fmt};

#[derive(Clone, Deserialize, Debug, Serialize)]
pub struct Metadata {
    // Name of the item.
    pub name: Option<String>,
    // A human readable description of the item. Markdown is supported.
    pub description: Option<String>,
    /// This is the URL to the image of the item. Can be just about any type of image (including SVGs, which will be cached into PNGs by OpenSea), and can be IPFS URLs or paths. We recommend using a 350 x 350 image.
    pub image: String,
    // This is the URL that will appear below the asset's image on OpenSea and will allow users to leave OpenSea and view the item on your site.
    pub external_url: Option<String>,
    // These are the attributes for the item, which will show up on the OpenSea page for the item. (see below)
    #[serde(deserialize_with = "sequence_or_map")]
    pub attributes: Vec<Attribute>,
    // Background color of the item on OpenSea. Must be a six-character hexadecimal without a pre-pended #.
    pub background_color: Option<String>,
    //
    pub created_by: Option<String>,
    // A URL to a multi-media attachment for the item. The file extensions GLTF, GLB, WEBM, MP4, M4V, OGV, and OGG are supported, along with the audio-only extensions MP3, WAV, and OGA.
    // Animation_url also supports HTML pages, allowing you to build rich experiences and interactive NFTs using JavaScript canvas, WebGL, and more. Scripts and relative paths within the HTML page are now supported. However, access to browser extensions is not supported.
    pub animation_url: Option<String>,
    // A URL to a YouTube video.
    pub youtube_url: Option<String>,

    pub uri: Option<String>,
    pub last_viewed: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug)]
pub enum Attribute {
    String {
        trait_type: String,
        value: String,
    },
    // Numeric
    Number {
        trait_type: String,
        value: i64,
        max_value: Option<usize>,
    },
    BoostPercentage {
        trait_type: String,
        value: f64,
        max_value: Option<usize>,
    },
    BoostNumber {
        trait_type: String,
        value: f64,
        max_value: Option<usize>,
    },
    // Date
    Date {
        trait_type: String,
        // A unix timestamp (seconds)
        value: u64,
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

const BOOST_NUMBER: &str = "boost_number";
const BOOST_PERCENTAGE: &str = "boost_percentage";
const DATE: &str = "date";
const DISPLAY_TYPE: &str = "display_type";
const MAX_VALUE: &str = "max_value";
const NUMBER: &str = "number";
const TRAIT_TYPE: &str = "trait_type";
const VALUE: &str = "value";

impl Serialize for Attribute {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Attribute::String { trait_type, value } => {
                let mut s = serializer.serialize_struct("Attribute", 2)?;
                s.serialize_field(TRAIT_TYPE, trait_type)?;
                s.serialize_field(VALUE, value)?;
                s.end()
            }
            Attribute::Number {
                trait_type,
                value,
                max_value,
            } => {
                let mut s = serializer.serialize_struct("Attribute", 4)?;
                s.serialize_field(DISPLAY_TYPE, NUMBER)?;
                s.serialize_field(TRAIT_TYPE, trait_type)?;
                s.serialize_field(VALUE, value)?;
                if let Some(max_value) = max_value {
                    s.serialize_field(MAX_VALUE, max_value)?
                }
                s.end()
            }
            Attribute::BoostPercentage {
                trait_type,
                value,
                max_value,
            } => {
                let mut s = serializer.serialize_struct("Attribute", 4)?;
                s.serialize_field(DISPLAY_TYPE, BOOST_PERCENTAGE)?;
                s.serialize_field(TRAIT_TYPE, trait_type)?;
                s.serialize_field(VALUE, value)?;
                if let Some(max_value) = max_value {
                    s.serialize_field(MAX_VALUE, max_value)?
                }
                s.end()
            }
            Attribute::BoostNumber {
                trait_type,
                value,
                max_value,
            } => {
                let mut s = serializer.serialize_struct("Attribute", 4)?;
                s.serialize_field(DISPLAY_TYPE, BOOST_PERCENTAGE)?;
                s.serialize_field(TRAIT_TYPE, trait_type)?;
                s.serialize_field(VALUE, value)?;
                if let Some(max_value) = max_value {
                    s.serialize_field(MAX_VALUE, max_value)?
                }
                s.end()
            }
            Attribute::Date { trait_type, value } => {
                let mut s = serializer.serialize_struct("Attribute", 3)?;
                s.serialize_field(DISPLAY_TYPE, DATE)?;
                s.serialize_field(TRAIT_TYPE, trait_type)?;
                s.serialize_field(VALUE, value)?;
                s.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for Attribute {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            #[serde(rename = "display_type")]
            DisplayType,
            #[serde(rename = "trait_type")]
            TraitType,
            Value,
            #[serde(rename = "max_value")]
            MaxValue,
        }

        struct DurationVisitor;

        impl<'de> Visitor<'de> for DurationVisitor {
            type Value = Attribute;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("enum Attribute")
            }

            fn visit_map<V: MapAccess<'de>>(self, mut map: V) -> Result<Attribute, V::Error> {
                let mut display_type = None;
                let mut trait_type = None;
                let mut value: Option<Value> = None;
                let mut max_value = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::DisplayType => {
                            if display_type.is_some() {
                                return Err(de::Error::duplicate_field(DISPLAY_TYPE));
                            }
                            display_type = Some(map.next_value()?);
                        }
                        Field::TraitType => {
                            if trait_type.is_some() {
                                return Err(de::Error::duplicate_field(TRAIT_TYPE));
                            }
                            trait_type = Some(map.next_value()?);
                        }
                        Field::Value => {
                            if value.is_some() {
                                return Err(de::Error::duplicate_field(VALUE));
                            }
                            value = Some(map.next_value()?);
                        }
                        Field::MaxValue => {
                            if max_value.is_some() {
                                return Err(de::Error::duplicate_field(MAX_VALUE));
                            }
                            max_value = Some(map.next_value()?);
                        }
                    }
                }
                let display_type = display_type.map_or("", |t| t);
                let trait_type = trait_type.ok_or_else(|| de::Error::missing_field(TRAIT_TYPE))?;
                let value = value.ok_or_else(|| de::Error::missing_field(VALUE))?;
                Ok(match display_type {
                    NUMBER => Attribute::Number {
                        trait_type,
                        value: value.as_i64().expect("could not convert value to number"),
                        max_value,
                    },
                    BOOST_PERCENTAGE => Attribute::BoostPercentage {
                        trait_type,
                        value: value.as_f64().expect("could not convert value to number"),
                        max_value,
                    },
                    BOOST_NUMBER => Attribute::BoostNumber {
                        trait_type,
                        value: value.as_f64().expect("could not convert value to number"),
                        max_value,
                    },
                    DATE => Attribute::Date {
                        trait_type,
                        value: value.as_u64().expect("could not convert value to number"),
                    },
                    &_ => {
                        let value = if value.is_string() {
                            value
                                .as_str()
                                .expect(&format!("could not convert {:?} value to string", value))
                                .to_string()
                        } else {
                            value.to_string()
                        };
                        Attribute::String { trait_type, value }
                    }
                })
            }
        }

        const FIELDS: &'static [&'static str] = &["secs", "nanos"];
        deserializer.deserialize_struct("Duration", FIELDS, DurationVisitor)
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

        fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
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

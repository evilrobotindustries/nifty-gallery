use serde::{
    de::{self},
    de::{MapAccess, SeqAccess, Visitor},
    ser::SerializeStruct,
    Deserialize, Deserializer, Serialize, Serializer,
};
use serde_json::Value;
use std::{fmt, marker::PhantomData};

const BOOST_NUMBER: &str = "boost_number";
const BOOST_PERCENTAGE: &str = "boost_percentage";
const DATE: &str = "date";
const DISPLAY_TYPE: &str = "display_type";
const MAX_VALUE: &str = "max_value";
const NUMBER: &str = "number";
const TRAIT_TYPE: &str = "trait_type";
const VALUE: &str = "value";

#[derive(Deserialize, Serialize)]
pub(crate) struct Metadata {
    pub name: Option<String>,
    pub description: Option<String>,
    pub image: String,
    pub external_url: Option<String>,
    #[serde(deserialize_with = "sequence_or_map")]
    pub attributes: Vec<Attribute>,
    pub background_color: Option<String>,
    pub created_by: Option<String>,
    pub animation_url: Option<String>,
    pub youtube_url: Option<String>,
}

pub(crate) enum Attribute {
    String {
        trait_type: String,
        value: String,
    },
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
    Date {
        trait_type: String,
        value: u64,
    },
}

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
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
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

        struct AttributeVisitor;

        impl<'de> Visitor<'de> for AttributeVisitor {
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

        const FIELDS: &'static [&'static str] =
            &["display_type", "trait_type", "value", "max_value"];
        deserializer.deserialize_struct("Attribute", FIELDS, AttributeVisitor)
    }
}

fn sequence_or_map<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<Attribute>, D::Error> {
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

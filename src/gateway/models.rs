use crate::errors::ApiError;
use serde::{
    de::{Error, MapAccess, SeqAccess, Unexpected, Visitor},
    ser::SerializeStruct,
    Deserialize, Deserializer, Serialize, Serializer,
};
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
pub struct MessagePayload {
    pub id: u64,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MessageIdPayload {
    pub id: u64,
}

#[derive(Debug, Clone)]
pub enum GatewayMessage<'a> {
    MessageCreated(MessagePayload),
    MessageUpdated(MessagePayload),
    MessageDelete(MessageIdPayload),
    Error(ApiError<'a>),
    Pong,
}

impl<'a> GatewayMessage<'a> {
    pub fn to_enum_str(&self) -> &'static str {
        match self {
            Self::MessageCreated(_) => "MESSAGE_CREATE",
            Self::MessageUpdated(_) => "MESSAGE_UPDATE",
            Self::MessageDelete(_) => "MESSAGE_DELETE",
            Self::Error(_) => "ERROR",
            Self::Pong => "PONG",
        }
    }
}

impl<'a> Serialize for GatewayMessage<'a> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s_ser = serializer.serialize_struct("GatewayMsg", 2)?;

        s_ser.serialize_field("type", self.to_enum_str())?;

        match self {
            Self::MessageCreated(p) => s_ser.serialize_field("data", p)?,
            Self::MessageUpdated(p) => s_ser.serialize_field("data", p)?,
            Self::MessageDelete(p) => s_ser.serialize_field("data", p)?,
            Self::Error(p) => s_ser.serialize_field("data", &p)?,
            Self::Pong => s_ser.serialize_field("data", &())?,
        };

        s_ser.end()
    }
}

#[derive(Debug)]
pub enum IncommingMessage {
    Ping,
}

impl<'de> Deserialize<'de> for IncommingMessage {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        const FIELDS: &'static [&'static str; 2] = &["type", "data"];

        enum FieldVariants {
            Type,
            Data,
        }
        struct FieldVisitor;

        impl<'de> Visitor<'de> for FieldVisitor {
            type Value = FieldVariants;

            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
                match v {
                    0 => Ok(FieldVariants::Type),
                    1 => Ok(FieldVariants::Data),
                    _ => Err(E::invalid_value(
                        serde::de::Unexpected::Unsigned(v),
                        &"field index 0 <= i < 2",
                    )),
                }
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                match v {
                    "type" => Ok(FieldVariants::Type),
                    "data" => Ok(FieldVariants::Data),
                    _ => Err(E::unknown_field(v, FIELDS)),
                }
            }

            fn visit_bytes<E: serde::de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
                match v {
                    b"type" => Ok(FieldVariants::Type),
                    b"data" => Ok(FieldVariants::Data),
                    _ => Err(E::unknown_field(&String::from_utf8_lossy(v), FIELDS)),
                }
            }

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("field identifier")
            }
        }

        impl<'de> Deserialize<'de> for FieldVariants {
            #[inline]
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        const KINDS: &'static [&'static str; 1] = &["PING"];

        struct ValueVisitor;

        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = IncommingMessage;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct IncommingMessage")
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let kind = seq.next_element::<&str>()?.ok_or(Error::invalid_length(
                    0,
                    &"struct MessagePayload with 2 elements",
                ))?;

                match kind {
                    "PING" => Ok(IncommingMessage::Ping),
                    _ => Err(Error::unknown_variant(kind, KINDS)),
                }
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut kind = Option::<String>::None;
                let mut data = Value::Null;

                while let Some((key, value)) = map.next_entry()? {
                    match key {
                        FieldVariants::Type => {
                            if kind.is_some() {
                                return Err(Error::duplicate_field("type"));
                            }

                            if let Value::String(s) = value {
                                kind = Some(s);
                            } else {
                                return Err(Error::invalid_type(
                                    Unexpected::Other("non-string type"),
                                    &"string",
                                ));
                            }
                        }
                        FieldVariants::Data => {
                            if !data.is_null() {
                                return Err(Error::duplicate_field("data"));
                            }
                            data = value
                        }
                    }
                }

                let kind = match kind {
                    Some(v) => v,
                    None => return Err(Error::missing_field("type")),
                };

                match kind.as_str() {
                    "PING" => Ok(IncommingMessage::Ping),
                    s => Err(Error::unknown_variant(s, KINDS)),
                }
            }
        }

        deserializer.deserialize_struct("IncommingMessage", FIELDS, ValueVisitor)
    }
}

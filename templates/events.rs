#[allow(dead_code)]
use std::convert::TryInto;

#[derive(::thiserror::Error, Debug)]
pub enum EventParseError {
    #[error("event topic mismatch")]
    TopicMismatch,
    #[error(transparent)]
    DecodeError(#[from] ::ethers::abi::Error),
}

{% for event in events %}
const {{ event.name | upper_snake }}_SIGNATURE: ::ethers::abi::Hash =
    ::ethers::types::H256(
        {{ event.signature }}
    );{% endfor %}

#[derive(Debug, Clone, ::serde::Serialize, ::serde::Deserialize)]
pub enum Events {
    {% for event in events %}{{ event.name | upper_camel }}({{ event.name | upper_camel }}),
    {% endfor %}
}

{% for event in events %}
#[derive(Debug, Clone, ::serde::Serialize, ::serde::Deserialize)]
pub struct {{ event.name | upper_camel }} {
    {% for input in event.inputs %}{{ input.name | lower_snake }}: {{ input.kind | normalize_type }},
    {% endfor %}
}
{% endfor %}

impl Events {
    pub fn signature(&self) -> ::ethers::abi::Hash {
        use Events::*;
        match self {
            {% for event in events %}{{ event.name | upper_camel }}(_) => {{ event.name | upper_snake }}_SIGNATURE,
            {% endfor %}
        }
    }
}

impl ::std::convert::TryFrom<::ethers::types::Log> for Events {
    type Error = EventParseError;

    fn try_from(log: ::ethers::types::Log) -> Result<Self, Self::Error> {
        use Events::*;
        let signature = log.topics[0];
        match log.topics[0] {
            {% for event in events %}_ if { signature == {{ event.name | upper_snake }}_SIGNATURE } => Ok({{ event.name | upper_camel }}(log.try_into()?)),
            {% endfor %}_ => Err(EventParseError::TopicMismatch)
        }
    }
}

{% for event in events %}
impl {{ event.name | upper_camel }} {
    pub fn signature() -> ::ethers::abi::Hash {
        {{ event.name | upper_snake }}_SIGNATURE
    }
}

impl ::std::convert::TryFrom<::ethers::types::Log> for {{ event.name | upper_camel }} {
    type Error = EventParseError;

    fn try_from(log: ::ethers::types::Log) -> Result<Self, Self::Error> {
        if !log.topics.iter().any(|t| *t == Self::signature()) {
            return Err(EventParseError::TopicMismatch)
        }
        let mut decoded = ::ethers::abi::decode(&[
            {% for input in event.inputs %}
            {{ input.kind | normalized_param_type }},
            {% endfor %}
        ], log.data.as_ref())?;
        Ok(Self {
            {% for input in event.inputs %}
            {{ input.name | lower_snake }}: decoded.remove(0){{ input.kind | normalized_parse }},
            {% endfor %}
        })
    }
}
{% endfor %}
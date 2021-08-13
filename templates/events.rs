#[allow(dead_code)]

#[derive(::thiserror::Error, Debug)]
pub enum EventParseError {
    #[error("event topic mismatch")]
    TopicMismatch,
    #[error(transparent)]
    DecodeError(#[from] ::ethers::abi::Error),
}

{% for event in events %}
static {{ event.name | upper_snake }}_SIGNATURE: [u8; 32] = {{ event.signature }};
{% endfor %}

#[derive(Debug, Clone, ::serde::Serialize, ::serde::Deserialize)]
pub enum Events {
    {% for event in events %}
    {{ event.name | upper_camel }}({{ event.name | upper_camel }}),
    {% endfor %}
}

{% for event in events %}
#[derive(Debug, Clone, ::serde::Serialize, ::serde::Deserialize)]
pub struct {{ event.name | upper_camel }} {
    {% for input in event.inputs %}
    {{ input.name | lower_snake }}: {{ input.kind | normalize_type }},
    {% endfor %}
}
{% endfor %}

impl Events {
    pub fn signature(&self) -> ::ethers::abi::Hash {
        use Events::*;
        match self {
            {% for event in events %}
            {{ event.name | upper_camel }}(_) => ::ethers::abi::Hash::from({{ event.name | upper_snake }}_SIGNATURE),
            {% endfor %}
        }
    }
}

{% for event in events %}
impl {{ event.name | upper_camel }} {
    pub fn signature() -> ::ethers::abi::Hash {
        ::ethers::abi::Hash::from({{ event.name | upper_snake }}_SIGNATURE)
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
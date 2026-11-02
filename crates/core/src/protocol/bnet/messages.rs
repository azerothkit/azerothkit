use anyhow::Result;
use prost::Message;

use super::generated::bgs::protocol;
use super::methods::Method;

pub(super) struct Attribute {
    pub name: String,
    pub blob: Option<Vec<u8>>,
}

pub(super) fn decode_attributes(
    method: Method,
    response: bool,
    bytes: &[u8],
) -> Result<Vec<Attribute>> {
    match (method, response) {
        (Method::GameUtilitiesV1, false) => {
            let message = protocol::game_utilities::v1::ClientRequest::decode(bytes)?;

            Ok(message.attribute.into_iter().map(from_v1).collect())
        }

        (Method::GameUtilitiesV1, true) => {
            let message = protocol::game_utilities::v1::ClientResponse::decode(bytes)?;

            Ok(message.attribute.into_iter().map(from_v1).collect())
        }

        (Method::GameUtilitiesV2, false) => {
            let message = protocol::game_utilities::v2::client::ProcessTaskRequest::decode(bytes)?;

            Ok(message.attribute.into_iter().map(from_v2).collect())
        }

        (Method::GameUtilitiesV2, true) => {
            let message = protocol::game_utilities::v2::client::ProcessTaskResponse::decode(bytes)?;

            Ok(message.result.into_iter().map(from_v2).collect())
        }
    }
}

fn from_v1(attribute: protocol::Attribute) -> Attribute {
    Attribute {
        name: attribute.name,
        blob: attribute.value.blob_value,
    }
}

fn from_v2(attribute: protocol::v2::Attribute) -> Attribute {
    let blob = attribute.value.and_then(|value| match value.r#type {
        Some(protocol::v2::variant::Type::BlobValue(bytes)) => Some(bytes),
        _ => None,
    });

    Attribute {
        name: attribute.name.unwrap_or_default(),
        blob,
    }
}

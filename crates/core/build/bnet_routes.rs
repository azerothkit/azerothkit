use std::collections::BTreeMap;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, bail, ensure};
use prost::Message;
use prost::encoding::{DecodeContext, WireType, decode_key, decode_varint, skip_field};
use prost_types::{DescriptorProto, FileDescriptorSet};

#[derive(Clone, PartialEq, Message)]
struct RawSet {
    #[prost(message, repeated, tag = "1")]
    file: Vec<RawFile>,
}

#[derive(Clone, PartialEq, Message)]
struct RawFile {
    #[prost(message, repeated, tag = "6")]
    service: Vec<RawService>,
}

#[derive(Clone, PartialEq, Message)]
struct RawService {
    #[prost(message, repeated, tag = "2")]
    method: Vec<RawMethod>,

    #[prost(bytes = "vec", tag = "3")]
    options: Vec<u8>,
}

#[derive(Clone, PartialEq, Message)]
struct RawMethod {
    #[prost(bytes = "vec", tag = "4")]
    options: Vec<u8>,
}

pub fn generate(bytes: &[u8], schemas: &FileDescriptorSet) -> Result<()> {
    let raw = RawSet::decode(bytes)?;

    let (service_extension, descriptor_name_field) =
        option_field(schemas, "bgs.protocol.service_options", "descriptor_name")?;

    let (method_extension, id_field) = option_field(schemas, "bgs.protocol.method_options", "id")?;

    let mut routes = BTreeMap::new();

    for (file, raw_file) in schemas.file.iter().zip(&raw.file) {
        for (service, raw_service) in file.service.iter().zip(&raw_file.service) {
            let service_name = format!("{}.{}", file.package(), service.name());

            let options = bytes_field(&raw_service.options, service_extension)?.unwrap_or_default();

            let alias = bytes_field(&options, descriptor_name_field)?
                .map(String::from_utf8)
                .transpose()?
                .unwrap_or_default();

            for (method, raw_method) in service.method.iter().zip(&raw_service.method) {
                let options =
                    bytes_field(&raw_method.options, method_extension)?.unwrap_or_default();

                let Some(id) = uint32_field(&options, id_field)? else {
                    continue;
                };

                let id = id & 0x3fff_ffff;

                let route = (
                    format!("{service_name}.{}", method.name()),
                    method.input_type().trim_start_matches('.').to_owned(),
                    method.output_type().trim_start_matches('.').to_owned(),
                );

                for name in [&service_name, &alias] {
                    if name.is_empty() {
                        continue;
                    }

                    let key = (fnv1a(name), id);

                    if let Some(previous) = routes.insert(key, route.clone()) {
                        ensure!(
                            previous == route,
                            "Ambiguous RPC route {key:?}: {} and {}",
                            previous.0,
                            route.0,
                        );
                    }
                }
            }
        }
    }

    ensure!(!routes.is_empty(), "No Battle.net RPC routes found");

    let mut output = String::from(
        "// Generated from bnet.protoset.\n\
         const ROUTES: &[Route] = &[\n",
    );

    for ((service_hash, method_id), (_, request, response)) in routes {
        writeln!(
            output,
            "Route {{ service_hash: {service_hash}, \
             method_id: {method_id}, \
             request: {request:?}, response: {response:?} }},",
        )?;
    }

    output.push_str("];\n");

    let directory = PathBuf::from(env::var_os("OUT_DIR").context("OUT_DIR is missing")?);

    fs::write(directory.join("bnet_routes.rs"), output)?;

    Ok(())
}

fn option_field(
    schemas: &FileDescriptorSet,
    extension_name: &str,
    field_name: &str,
) -> Result<(u32, u32)> {
    for file in &schemas.file {
        for extension in &file.extension {
            let name = format!("{}.{}", file.package(), extension.name());

            if name != extension_name {
                continue;
            }

            let target = extension.type_name().trim_start_matches('.');

            let message = schemas
                .file
                .iter()
                .find_map(|file| find_message(&file.message_type, file.package(), target))
                .with_context(|| format!("Missing option message: {target}"))?;

            let field = message
                .field
                .iter()
                .find(|field| field.name() == field_name)
                .with_context(|| format!("Missing option field: {target}.{field_name}"))?;

            let extension_number = u32::try_from(extension.number())?;
            let field_number = u32::try_from(field.number())?;

            ensure!(
                extension_number > 0 && field_number > 0,
                "Invalid protobuf option field number",
            );

            return Ok((extension_number, field_number));
        }
    }

    bail!("Missing protobuf extension: {extension_name}")
}

fn find_message<'a>(
    messages: &'a [DescriptorProto],
    scope: &str,
    target: &str,
) -> Option<&'a DescriptorProto> {
    for message in messages {
        let name = if scope.is_empty() {
            message.name().to_owned()
        } else {
            format!("{scope}.{}", message.name())
        };

        if name == target {
            return Some(message);
        }

        if let Some(found) = find_message(&message.nested_type, &name, target) {
            return Some(found);
        }
    }

    None
}

fn bytes_field(mut input: &[u8], wanted: u32) -> Result<Option<Vec<u8>>> {
    let mut result = None;

    while !input.is_empty() {
        let (tag, wire_type) = decode_key(&mut input)?;

        if tag == wanted {
            ensure!(
                wire_type == WireType::LengthDelimited,
                "Expected bytes for protobuf option field {wanted}",
            );

            let size = usize::try_from(decode_varint(&mut input)?)?;
            ensure!(size <= input.len(), "Truncated protobuf option");

            result = Some(input[..size].to_vec());
            input = &input[size..];
        } else {
            skip_field(wire_type, tag, &mut input, DecodeContext::default())?;
        }
    }

    Ok(result)
}

fn uint32_field(mut input: &[u8], wanted: u32) -> Result<Option<u32>> {
    let mut result = None;

    while !input.is_empty() {
        let (tag, wire_type) = decode_key(&mut input)?;

        if tag == wanted {
            ensure!(
                wire_type == WireType::Varint,
                "Expected uint32 for protobuf option field {wanted}",
            );

            result = Some(u32::try_from(decode_varint(&mut input)?)?);
        } else {
            skip_field(wire_type, tag, &mut input, DecodeContext::default())?;
        }
    }

    Ok(result)
}

fn fnv1a(value: &str) -> u32 {
    value.as_bytes().iter().fold(0x811c_9dc5, |hash, byte| {
        (hash ^ u32::from(*byte)).wrapping_mul(0x0100_0193)
    })
}

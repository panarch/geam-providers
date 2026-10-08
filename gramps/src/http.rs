mod parser;
mod schema;

use crate::{Call, Component, GrampsProfile};
use geam::gleam_stdlib::provider_support::{Dynamic, DynamicSchema, GleamError, GleamOk};
use geam::host::{
    HostCallCompletion, HostCallError, HostConstructions, HostCustom, HostList, HostListType,
    HostProviderModule, HostRegistrationError, HostTupleType, HostTypeIndex0, HostTypeIndexNext,
    HostTypeList, HostTypeParameter,
};
use geam::provider::{BitArrayValue, HostFailure, StringValue};
use schema::{
    DecodeError, DecodeResult, DecodedPacket, One, PacketType, Scheme, Two, Uri, Version,
};

type Pair = HostTupleType<Two<DecodedPacket, BitArrayValue>>;
type Targets = HostTypeList<
    DecodedPacket,
    HostTypeList<
        Uri,
        HostTypeList<
            Scheme,
            HostTypeList<Version, HostTypeList<DecodeError, HostTypeList<Dynamic, One<Pair>>>>,
        >,
    >,
>;
type I0 = HostTypeIndex0;
type I1 = HostTypeIndexNext<I0>;
type I2 = HostTypeIndexNext<I1>;
type I3 = HostTypeIndexNext<I2>;
type I4 = HostTypeIndexNext<I3>;
type I5 = HostTypeIndexNext<I4>;
type I6 = HostTypeIndexNext<I5>;

pub(super) fn provider<Profile: GrampsProfile>()
-> Result<HostProviderModule<Profile>, HostRegistrationError> {
    HostProviderModule::new("gramps", "gramps/http").and_then(|module| {
        module.with_scoped_function_and_constructions::<Component, (
            PacketType,
            BitArrayValue,
            HostListType<HostTypeParameter<0>>,
        ), DecodeResult, Targets, _>("decode_packet", decode::<Profile>)
    })
}

fn decode<'call, Profile: GrampsProfile>(
    mut call: Call<'call, Profile, DecodeResult>,
    tokens: HostConstructions<'call, Targets>,
    kind: HostCustom<'call, PacketType>,
    data: BitArrayValue,
    options: HostList<'call, HostTypeParameter<0>>,
) -> Result<HostCallCompletion<'call, DecodeResult>, HostCallError> {
    if call.list_item::<HostTypeParameter<0>>(options, 0).is_some() {
        return Err(HostFailure::new("gramps HTTP decoder requires empty options").into());
    }
    let headers = call.custom_fields::<schema::HeaderKind>(kind).is_some();
    let parsed = if data.bit_len().is_multiple_of(8) {
        parser::decode(data.bytes(), headers)
    } else {
        Err(parser::Error::Invalid(
            "HTTP input must contain complete bytes",
        ))
    };
    let (packet, consumed) = match parsed {
        Ok(value) => value,
        Err(error) => {
            let error = match error {
                parser::Error::More => {
                    call.construct_custom::<schema::Incomplete>(tokens.at::<I4>(), (0.into(), ()))
                }
                parser::Error::Invalid(reason) => {
                    call.construct_custom::<schema::Invalid>(tokens.at::<I4>(), (reason.into(), ()))
                }
            };
            return Ok(call.return_custom::<GleamError<Pair, DecodeError>>((error, ())));
        }
    };
    let packet = match packet {
        parser::Packet::Request {
            method,
            uri,
            version,
        } => {
            let uri = match uri {
                parser::Uri::Origin(path) => call
                    .construct_custom::<schema::OriginUri>(tokens.at::<I1>(), (string(path), ())),
                parser::Uri::Absolute {
                    https,
                    host,
                    port,
                    path,
                    query_only,
                } => {
                    let scheme = if https {
                        call.construct_custom::<schema::HttpsScheme>(tokens.at::<I2>(), ())
                    } else {
                        call.construct_custom::<schema::HttpScheme>(tokens.at::<I2>(), ())
                    };
                    let path = if path.is_empty() {
                        "/".into()
                    } else if query_only {
                        let mut bytes = vec![b'/'];
                        bytes.extend_from_slice(path);
                        StringValue::from_bytes(bytes)
                    } else {
                        string(path)
                    };
                    call.construct_custom::<schema::Absolute>(
                        tokens.at::<I1>(),
                        (scheme, (string(host), (port.into(), (path, ())))),
                    )
                }
            };
            let version = call.construct_tuple(
                tokens.at::<I3>(),
                (version.0.into(), (version.1.into(), ())),
            );
            call.construct_custom::<schema::Request>(
                tokens.at::<I0>(),
                (string(method), (uri, (version, ()))),
            )
        }
        parser::Packet::Response {
            version,
            status,
            text,
        } => {
            let version = call.construct_tuple(
                tokens.at::<I3>(),
                (version.0.into(), (version.1.into(), ())),
            );
            call.construct_custom::<schema::Response>(
                tokens.at::<I0>(),
                (version, (status.into(), (string(text), ()))),
            )
        }
        parser::Packet::Header { field, value } => {
            let raw = string(field);
            let native = call.native_value::<StringValue>(raw.clone());
            let dynamic = call.construct_external_with_binding::<geam::gleam_erlang::Component<Profile>,DynamicSchema,geam::HostTypeListEnd>(tokens.at::<I5>(),geam::gleam_stdlib::Dynamic::from_native(native));
            call.construct_custom::<schema::Header>(
                tokens.at::<I0>(),
                (0.into(), (dynamic, (raw, (string(value), ())))),
            )
        }
        parser::Packet::Eoh => call.construct_custom::<schema::Eoh>(tokens.at::<I0>(), ()),
    };
    // BitArrayValue has no public retained range constructor at this revision.
    // Construct the newly returned remainder once; never reparse or consume its body.
    let rest = BitArrayValue::from_bytes(data.bytes()[consumed..].to_vec());
    let result = call.construct_tuple(tokens.at::<I6>(), (packet, (rest, ())));
    Ok(call.return_custom::<GleamOk<Pair, DecodeError>>((result, ())))
}

fn string(bytes: &[u8]) -> StringValue {
    StringValue::from_bytes(bytes.to_vec())
}

#[cfg(test)]
mod tests {
    #[test]
    fn original_generic_options_fail_at_the_native_boundary_before_parsing() {
        let additional = r#"
pub fn probe_options() { let _=decode_packet(HttpBin,<<>>,[1]) Nil }
"#;
        let (mut execution, mut state) = crate::test_support::source_project(
            "import gramps/http\npub fn main() { http.probe_options() }",
            &[("gramps/http", additional)],
            [],
        );
        let error =
            crate::test_support::execution_host::run(&mut execution, &mut state, &mut Vec::new())
                .unwrap_err();
        assert_eq!(
            error.to_string(),
            "host function gramps::gramps/http.decode_packet failed: gramps HTTP decoder requires empty options"
        );
    }
}

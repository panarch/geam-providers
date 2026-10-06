use crate::schema::{ErlangTcpOption, ErlangTcpOptionSchema, Four, IpAddress, One, TcpOption, Two};
use crate::{A, Call, Component, GlistenProfile};
use geam::gleam_erlang::{Charlist, service};
use geam::gleam_stdlib::provider_support::{Dynamic, DynamicSchema, GleamResult};
use geam::host::native::{NativeCall, NativeRules};
use geam::host::{
    HostCallCompletion, HostCallError, HostComponentProfile, HostConstructions, HostExternal,
    HostExternalBinding, HostExternalEquality, HostExternalHashing, HostExternalInspection,
    HostExternalStorage, HostExternalStore, HostList, HostListType, HostProviderModule,
    HostRegistrationError, HostTypeIndex0, HostTypeIndexNext, HostTypeListEnd,
};
use geam::provider::HostFailure;
use geam::provider::advanced::{NativeKind, NativeMap, NativeMapEntry, NativeValue};
use std::net::IpAddr;
use std::sync::Arc;

// One typed declaration owns the unchanged source constructors and codecs.
#[geam::module(path = "glisten/socket/options", crate_path = geam,
    profile = crate::GlistenProfile, component = crate::Component)]
#[allow(
    dead_code,
    reason = "owned enums define source schemas; this adapter consumes their generated input forms"
)]
pub mod types {
    use geam::provider::{BigInt, StringValue};

    #[geam::custom(input = SocketModeInput)]
    pub enum SocketMode {
        Binary,
    }
    #[geam::custom(input = ActiveStateInput)]
    pub enum ActiveState {
        Once,
        Passive,
        Count(BigInt),
        Active,
    }
    #[geam::custom(input = IpAddressInput)]
    pub enum IpAddress {
        IpV4(BigInt, BigInt, BigInt, BigInt),
        IpV6(
            BigInt,
            BigInt,
            BigInt,
            BigInt,
            BigInt,
            BigInt,
            BigInt,
            BigInt,
        ),
    }
    #[geam::custom(input = InterfaceInput)]
    #[allow(
        clippy::large_enum_variant,
        reason = "the source schema declares an inline BigInt address; generated typed input retains that exact shape"
    )]
    pub enum Interface {
        Address(IpAddress),
        Any,
        Loopback,
    }
    #[geam::custom(input = TlsCertsInput)]
    pub enum TlsCerts {
        CertKeyFiles {
            certfile: StringValue,
            keyfile: StringValue,
        },
    }
    #[geam::custom(input = TcpOptionInput)]
    pub enum TcpOption {
        Backlog(BigInt),
        Nodelay(bool),
        Linger((bool, BigInt)),
        SendTimeout(BigInt),
        SendTimeoutClose(bool),
        Reuseaddr(bool),
        ActiveMode(ActiveState),
        Mode(SocketMode),
        CertKeyConfig(TlsCerts),
        AlpnPreferredProtocols(Vec<StringValue>),
        Ipv6,
        Buffer(BigInt),
        Ip(Interface),
    }
}

type I0 = HostTypeIndex0;
type I1 = HostTypeIndexNext<I0>;
type I2 = HostTypeIndexNext<I1>;
type I3 = HostTypeIndexNext<I2>;
type Items = HostListType<A>;
type OptionList = HostListType<ErlangTcpOption>;
type OptionConstructions = Four<OptionList, ErlangTcpOption, HostListType<Dynamic>, Dynamic>;
type ParseResult = GleamResult<A, ()>;
type ParseTargets = Two<ParseResult, IpAddress>;

/// One typed source option and its retained Erlang view share the same owner.
/// The generated List input retains ALPN storage and decodes items on demand.
pub struct ErlangOption {
    pub(crate) native: NativeValue,
    pub(crate) typed: types::TcpOptionInput,
}

pub struct OptionStorage;
impl<Profile: GlistenProfile> HostExternalBinding<Profile, ErlangTcpOptionSchema> for Component {
    type Storage = OptionStorage;
}
impl<Profile: GlistenProfile> HostExternalStorage<Profile, ErlangTcpOptionSchema>
    for OptionStorage
{
    type Payload = ErlangOption;
    fn store(stores: &Profile::ExternalStores) -> &HostExternalStore<Self::Payload> {
        &<Profile as HostComponentProfile<Component>>::component_stores(stores).options
    }
    fn source_equal(
        context: &HostExternalEquality<'_>,
        left: &Self::Payload,
        right: &Self::Payload,
    ) -> bool {
        left.native.source_equal(context, &right.native)
    }
    fn source_hash(context: &HostExternalHashing<'_>, value: &Self::Payload) -> u64 {
        value.native.source_hash(context)
    }
    fn inspect(
        context: &HostExternalInspection<'_>,
        value: &Self::Payload,
    ) -> geam::provider::EcoString {
        value.native.inspect(context)
    }
    fn native_view(value: &Self::Payload) -> Option<NativeValue> {
        Some(value.native.clone())
    }
}

pub(crate) fn provider<Profile: GlistenProfile>()
-> Result<HostProviderModule<Profile>, HostRegistrationError> {
    types::__geam_module::<Profile>()
        .and_then(|module| module.with_external_type::<Component, ErlangTcpOptionSchema>())
        .and_then(|module| module.with_scoped_function_and_constructions::<Component, (HostListType<TcpOption>,), OptionList, OptionConstructions, _>("to_erl_options", to_erl_options::<Profile>))
        .and_then(|module| module.with_native_function::<Component, (Items, Items), Items, One<Items>, _>("merge_type_list", NativeRules::default(), merge::<Profile>))
}

pub(crate) fn address_provider<Profile: GlistenProfile>()
-> Result<HostProviderModule<Profile>, HostRegistrationError> {
    HostProviderModule::new("glisten", "glisten").and_then(|module| {
        module.with_native_function::<Component, (Charlist,), ParseResult, ParseTargets, _>(
            "parse_address",
            NativeRules::default(),
            parse::<Profile>,
        )
    })
}

fn parse<'call, Profile: GlistenProfile>(
    mut native: NativeCall<'call, Profile, Component, ParseResult, ParseTargets>,
    text: HostExternal<'call, Charlist>,
) -> Result<HostCallCompletion<'call, ParseResult>, HostCallError> {
    let text = service::charlist_string(native.call(), text);
    // The Charlist producer constructs Unicode text; Display preserves that text.
    let value = match text.to_string().parse::<IpAddr>() {
        Ok(address) => {
            let tag = if address.is_ipv4() { "ip_v4" } else { "ip_v6" };
            let values = native.call().native_values();
            let address = NativeValue::tuple(
                std::iter::once(NativeValue::symbol(tag)).chain(
                    crate::network::ip_address(address)
                        .into_iter()
                        .map(|part| values.integer(part.into())),
                ),
            );
            NativeValue::tuple([NativeValue::symbol("ok"), address])
        }
        Err(_) => NativeValue::tuple([NativeValue::symbol("error"), NativeValue::symbol("nil")]),
    };
    let result = native
        .convert::<I0>(&value)
        .ok_or_else(|| HostFailure::new("parse_address return type does not match IpAddress"))?;
    Ok(native.finish(result))
}

fn key(value: &NativeValue) -> Result<NativeValue, HostCallError> {
    match value.kind() {
        NativeKind::Symbol => Ok(value.clone()),
        NativeKind::Tuple => value
            .index(0)
            .ok_or_else(|| HostFailure::new("option tuple has no type key").into()),
        _ => Err(HostFailure::new("option requires a symbol or tuple type key").into()),
    }
}

fn merge<'call, Profile: GlistenProfile>(
    mut native: NativeCall<'call, Profile, Component, Items, One<Items>>,
    original: HostList<'call, A>,
    overrides: HostList<'call, A>,
) -> Result<HostCallCompletion<'call, Items>, HostCallError> {
    let mut replacements = Vec::new();
    let mut keys = Vec::new();
    let mut index = 0;
    while let Some(value) = native.call().list_item::<A>(overrides, index) {
        keys.push(key(&native.source::<A>(value))?);
        replacements.push(value);
        index += 1;
    }
    let mut retained = Vec::new();
    index = 0;
    while let Some(value) = native.call().list_item::<A>(original, index) {
        let own = key(&native.source::<A>(value))?;
        if !keys
            .iter()
            .any(|other| native.call().native_equal(&own, other))
        {
            retained.push(value);
        }
        index += 1;
    }
    // The upstream FFI's fold prepends each retained original before overrides.
    retained.reverse();
    retained.extend(replacements);
    let (mut call, constructions) = native.into_call();
    let result = call.construct_list(constructions.at::<I0>(), retained);
    Ok(call.return_value(result))
}

fn to_erl_options<'call, Profile: GlistenProfile>(
    mut call: Call<'call, Profile, OptionList>,
    constructions: HostConstructions<'call, OptionConstructions>,
    options: HostList<'call, TcpOption>,
) -> Result<HostCallCompletion<'call, OptionList>, HostCallError> {
    let mut result = Vec::new();
    let mut index = 0;
    while let Some(option) = call.list_item::<TcpOption>(options, index) {
        let converted = convert_option(
            &mut call,
            constructions.at::<I2>(),
            constructions.at::<I3>(),
            option,
        );
        result.push(call.construct_external(constructions.at::<I1>(), converted));
        index += 1;
    }
    let result = call.construct_list(constructions.at::<I0>(), result);
    Ok(call.return_value(result))
}

fn convert_option<'call, Profile: GlistenProfile, Return: geam::host::HostType>(
    call: &mut Call<'call, Profile, Return>,
    dynamic_list: geam::host::HostConstruction<'call, HostListType<Dynamic>>,
    dynamic: geam::host::HostConstruction<'call, Dynamic>,
    option: geam::host::HostCustom<'call, TcpOption>,
) -> ErlangOption {
    let original = call.native_value::<TcpOption>(option);
    let typed = <types::TcpOptionInput as geam::__macro_support::ProviderInputValue<
        Profile,
        Component,
        Return,
    >>::from_host(call, option);
    let converted = match &typed {
        types::TcpOptionInput::ActiveMode(state) => {
            let mode = match state {
                types::ActiveStateInput::Once => NativeValue::symbol("once"),
                types::ActiveStateInput::Passive => NativeValue::symbol("false"),
                types::ActiveStateInput::Active => NativeValue::symbol("true"),
                types::ActiveStateInput::Count(count) => {
                    call.native_values().integer(count.clone())
                }
            };
            NativeValue::tuple([NativeValue::symbol("active"), mode])
        }
        types::TcpOptionInput::Ip(interface) => {
            let value = match interface {
                types::InterfaceInput::Any => NativeValue::symbol("any"),
                types::InterfaceInput::Loopback => NativeValue::symbol("loopback"),
                types::InterfaceInput::Address(address) => {
                    let parts = match address {
                        types::IpAddressInput::IpV4(a, b, c, d) => vec![a, b, c, d],
                        types::IpAddressInput::IpV6(a, b, c, d, e, f, g, h) => {
                            vec![a, b, c, d, e, f, g, h]
                        }
                    };
                    NativeValue::tuple(
                        parts
                            .into_iter()
                            .map(|part| call.native_values().integer(part.clone())),
                    )
                }
            };
            NativeValue::tuple([NativeValue::symbol("ip"), value])
        }
        types::TcpOptionInput::Ipv6 => NativeValue::symbol("inet6"),
        types::TcpOptionInput::CertKeyConfig(types::TlsCertsInput::CertKeyFiles {
            certfile,
            keyfile,
        }) => {
            let entries = vec![
                (
                    NativeValue::symbol("certfile"),
                    call.native_values().string(certfile.clone()),
                ),
                (
                    NativeValue::symbol("keyfile"),
                    call.native_values().string(keyfile.clone()),
                ),
            ];
            let entries: Arc<[NativeMapEntry]> = entries
                .into_iter()
                .map(|(key, value)| NativeMapEntry {
                    key_hash: call.native_hash(&key),
                    key,
                    value,
                })
                .collect();
            let map = NativeValue::map(NativeMap::new(
                entries,
                |entries| entries.len(),
                |entries| {
                    entries
                        .iter()
                        .map(|entry| NativeMapEntry {
                            key_hash: entry.key_hash,
                            key: entry.key.clone(),
                            value: entry.value.clone(),
                        })
                        .collect::<Vec<_>>()
                        .into_iter()
                },
                |entries, hash, key, equal| {
                    entries.iter().find_map(|entry| {
                        (entry.key_hash == hash && equal(&entry.key, key))
                            .then(|| entry.value.clone())
                    })
                },
            ));
            let value = call.construct_external_with_binding::<geam::gleam_erlang::Component<Profile>, DynamicSchema, HostTypeListEnd>(dynamic, geam::gleam_stdlib::Dynamic::from_native(map));
            let list = call.construct_list(dynamic_list, [value]);
            NativeValue::tuple([
                NativeValue::symbol("certs_keys"),
                call.native_value::<HostListType<Dynamic>>(list),
            ])
        }
        _ => original,
    };
    ErlangOption {
        native: converted,
        typed,
    }
}

#[cfg(test)]
mod tests {
    use super::{ErlangOption, OptionStorage};
    use crate::network::Network;
    use crate::test_support::{
        execution_fixture::TestHost, network::ScriptedNetwork, source_project,
    };
    use crate::{
        Call, Component,
        schema::{ErlangTcpOptionSchema, TcpOption, Three},
        test_support::{Profile, Stores, source_project_with},
    };
    use geam::gleam_stdlib::provider_support::Dynamic;
    use geam::host::{
        HostCallCompletion, HostCallError, HostConstructions, HostCustom, HostExternal,
        HostExternalBinding, HostExternalEquality, HostExternalHashing, HostExternalInspection,
        HostExternalSchema, HostExternalStorage, HostExternalStore, HostExternalType, HostListType,
        HostProviderModule, HostTypeListEnd,
    };
    use std::net::{Ipv4Addr, SocketAddr};
    use std::sync::Arc;

    // The production payload remains in its original store. Removing only its
    // native-view shortcut makes source operations exercise storage callbacks.
    struct Probe;
    type ProbeValue = HostExternalType<Probe>;
    type ProbeTargets = Three<ProbeValue, HostListType<Dynamic>, Dynamic>;
    impl HostExternalSchema for Probe {
        const PACKAGE: &'static str = "fixture";
        const MODULE: &'static str = "fixture";
        const NAME: &'static str = "Probe";
        const PARAMETER_COUNT: usize = 0;
    }
    impl HostExternalBinding<Profile, Probe> for Component {
        type Storage = Probe;
    }
    impl HostExternalStorage<Profile, Probe> for Probe {
        type Payload = ErlangOption;
        fn store(stores: &Stores) -> &HostExternalStore<ErlangOption> {
            <OptionStorage as HostExternalStorage<Profile, ErlangTcpOptionSchema>>::store(stores)
        }
        fn source_equal(
            context: &HostExternalEquality<'_>,
            left: &ErlangOption,
            right: &ErlangOption,
        ) -> bool {
            <OptionStorage as HostExternalStorage<Profile, ErlangTcpOptionSchema>>::source_equal(
                context, left, right,
            )
        }
        fn source_hash(context: &HostExternalHashing<'_>, value: &ErlangOption) -> u64 {
            <OptionStorage as HostExternalStorage<Profile, ErlangTcpOptionSchema>>::source_hash(
                context, value,
            )
        }
        fn inspect(
            context: &HostExternalInspection<'_>,
            value: &ErlangOption,
        ) -> geam::provider::EcoString {
            <OptionStorage as HostExternalStorage<Profile, ErlangTcpOptionSchema>>::inspect(
                context, value,
            )
        }
    }
    fn make_probe<'call>(
        mut call: Call<'call, Profile, ProbeValue>,
        constructions: HostConstructions<'call, ProbeTargets>,
        option: HostCustom<'call, TcpOption>,
    ) -> Result<HostCallCompletion<'call, ProbeValue>, HostCallError> {
        let payload = super::convert_option(
            &mut call,
            constructions.at::<super::I1>(),
            constructions.at::<super::I2>(),
            option,
        );
        let value = call.construct_external(constructions.at::<super::I0>(), payload);
        Ok(call.return_value(value))
    }
    fn check_probe<'call>(
        call: Call<'call, Profile, ()>,
        a: HostExternal<'call, ProbeValue>,
        b: HostExternal<'call, ProbeValue>,
        c: HostExternal<'call, ProbeValue>,
        expected: geam::provider::StringValue,
    ) -> Result<HostCallCompletion<'call, ()>, HostCallError> {
        assert!(call.equal::<ProbeValue>(a, b));
        assert!(!call.equal::<ProbeValue>(a, c));
        assert_eq!(
            call.source_hash::<ProbeValue>(a),
            call.source_hash::<ProbeValue>(b)
        );
        assert_eq!(
            call.inspect::<ProbeValue>(a).as_str(),
            expected.as_str().expect("Unicode inspection")
        );
        let native = |value| {
            <OptionStorage as HostExternalStorage<Profile, ErlangTcpOptionSchema>>::native_view(
                &call.external_payload::<Probe, HostTypeListEnd>(value),
            )
            .unwrap()
        };
        let (a, b, c) = (native(a), native(b), native(c));
        assert!(call.native_equal(&a, &b));
        assert!(!call.native_equal(&a, &c));
        assert_eq!(call.native_hash(&a), call.native_hash(&b));
        Ok(call.return_value(()))
    }

    #[test]
    fn option_storage_callbacks_agree_with_the_original_native_view_without_cloning_payloads() {
        let provider = HostProviderModule::new("fixture", "fixture").unwrap()
            .with_external_type::<Component, Probe>().unwrap()
            .with_scoped_function_and_constructions::<Component, (TcpOption,), ProbeValue, ProbeTargets, _>("make", make_probe).unwrap()
            .with_scoped_function::<Component, (ProbeValue, ProbeValue, ProbeValue, geam::provider::StringValue), (), _>("check", check_probe).unwrap();
        let source = r#"
import gleam/dict
import gleam/list
import gleam/string
import glisten/socket/options as options
pub type Probe
@external(erlang, "fixture", "make") fn make(value: options.TcpOption) -> Probe
@external(erlang, "fixture", "check") fn check(a: Probe, b: Probe, c: Probe, expected: String) -> Nil
fn verify(a, b) {
  let first = make(a)
  let same = make(a)
  let different = make(b)
  let assert Ok(expected) = list.first(options.to_erl_options([a]))
  check(first, same, different, string.inspect(expected))
  let table = dict.from_list([#(first, 42)])
  assert dict.get(table, same) == Ok(42)
  assert dict.get(table, different) == Error(Nil)
}
pub fn main() {
  verify(options.Backlog(1), options.Backlog(2))
  verify(options.CertKeyConfig(options.CertKeyFiles("a.pem", "key.pem")), options.CertKeyConfig(options.CertKeyFiles("b.pem", "key.pem")))
  verify(options.AlpnPreferredProtocols(["h2", "http/1.1"]), options.AlpnPreferredProtocols(["http/1.1", "h2"]))
  Nil
}
"#;
        let network = Arc::new(ScriptedNetwork::new(
            "127.0.0.1:4321".parse().unwrap(),
            vec![],
        ));
        let (mut execution, mut state) = source_project_with(source, network, [provider]);
        let host = TestHost::default();
        assert_eq!(
            host.block_on(execution.run_main(&host, &mut state, &mut Vec::new()))
                .map(|outcome| outcome
                    .try_into_value()
                    .expect("fixture must return normally"))
                .unwrap(),
            geam::Value::Nil
        );
    }

    #[test]
    fn generic_parse_preserves_original_charlist_signature_and_return_specialization() {
        use crate::{
            Component,
            schema::Two,
            test_support::{Profile, source_project_with},
        };
        use geam::gleam_erlang::Charlist;
        use geam::host::{HostProviderModule, native::NativeRules};
        for (address, specialization, expected) in [
            ("127.0.0.1", "options.IpAddress", None),
            ("2001:db8::1", "options.IpAddress", None),
            (
                "127.0.0.1",
                "Int",
                Some("parse_address return type does not match IpAddress"),
            ),
            ("not an address", "Int", None),
        ] {
            let provider = HostProviderModule::new("fixture", "fixture").unwrap()
                .with_native_function::<Component, (Charlist,), super::ParseResult, Two<super::ParseResult, crate::schema::IpAddress>, _>("parse", NativeRules::default(), super::parse::<Profile>).unwrap();
            let source = format!(
                r#"
import gleam/erlang/charlist
import glisten/socket/options
@external(erlang, "glisten_ffi", "parse_address")
fn parse(value: charlist.Charlist) -> Result(a, Nil)
pub fn main() {{
  let parsed: Result({specialization}, Nil) = parse(charlist.from_string("{address}"))
  {assertion}
  Nil
}}
"#,
                assertion = if address == "not an address" {
                    "assert parsed == Error(Nil)"
                } else {
                    "let assert Ok(_) = parsed"
                }
            );
            let network = Arc::new(ScriptedNetwork::new(
                "127.0.0.1:4321".parse().unwrap(),
                vec![],
            ));
            let (mut execution, mut state) = source_project_with(&source, network, [provider]);
            let host = TestHost::default();
            let actual = host
                .block_on(execution.run_main(&host, &mut state, &mut Vec::new()))
                .map(|outcome| {
                    outcome
                        .try_into_value()
                        .expect("fixture must return normally")
                });
            match expected {
                None => assert_eq!(actual.unwrap(), geam::Value::Nil),
                Some(message) => assert!(actual.unwrap_err().to_string().contains(message)),
            }
        }
    }

    #[test]
    fn original_options_keep_type_keys_override_order_and_captured_values() {
        let network: Arc<dyn Network> = Arc::new(ScriptedNetwork::new(
            SocketAddr::from((Ipv4Addr::LOCALHOST, 4321)),
            vec![],
        ));
        let (mut execution, mut state) = source_project(
            r#"
import gleam/dict
import glisten/socket/options as options
pub type Choice { First(fn() -> Int) Second(Int) Empty }
pub fn main() {
  assert options.merge_type_list([#("a", 1), #("b", 2), #("c", 3)], [#("b", 20)])
    == [#("c", 3), #("a", 1), #("b", 20)]
  assert options.merge_type_list([Empty, Second(1)], [Second(2)]) == [Empty, Second(2)]
  let assert [First(retained), Second(2)] =
    options.merge_type_list([First(fn() { 7 }), Second(1)], [Second(2)])
  assert retained() == 7
  assert options.merge_type_list([], [options.Nodelay(False)]) == [options.Nodelay(False)]
  assert options.merge_type_list([options.Buffer(8), options.Nodelay(True)], [])
    == [options.Nodelay(True), options.Buffer(8)]
  let original = [options.ActiveMode(options.Passive), options.CertKeyConfig(options.CertKeyFiles("cert.pem", "key.pem"))]
  let first = options.to_erl_options(original)
  let second = options.to_erl_options(original)
  assert first == second
  let hashed = dict.from_list([#(first, 42)])
  let assert Ok(42) = dict.get(hashed, second)
  let assert Error(Nil) = dict.get(hashed, options.to_erl_options([options.ActiveMode(options.Once)]))
  Nil
}
"#,
            network,
        );
        let host = TestHost::default();
        assert_eq!(
            host.block_on(execution.run_main(&host, &mut state, &mut Vec::new()))
                .map(|outcome| outcome
                    .try_into_value()
                    .expect("fixture must return normally"))
                .unwrap(),
            geam::Value::Nil
        );
    }

    #[test]
    fn generic_merge_reports_unsupported_keys_as_host_failures() {
        for source in [
            "import glisten/socket/options\npub fn main() { options.merge_type_list([1], []) }",
            "import glisten/socket/options\npub fn main() { options.merge_type_list([], [\"invalid\"]) }",
            "import gleam/dynamic\nimport glisten/socket/options\npub fn main() { options.merge_type_list([dynamic.array([])], []) }",
        ] {
            let network: Arc<dyn Network> = Arc::new(ScriptedNetwork::new(
                SocketAddr::from((Ipv4Addr::LOCALHOST, 4321)),
                vec![],
            ));
            let (mut execution, mut state) = source_project(source, network);
            let host = TestHost::default();
            let failure = host
                .block_on(execution.run_main(&host, &mut state, &mut Vec::new()))
                .unwrap_err();
            assert!(
                failure
                    .to_string()
                    .contains(if source.contains("dynamic.array") {
                        "option tuple has no type key"
                    } else {
                        "option requires a symbol or tuple type key"
                    }),
                "{failure}"
            );
        }
    }
}

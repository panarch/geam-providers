use crate::schema::{
    self, ConnectError, DefaultConstructions, ErlOption, ErrorConstructions, Header, Headers,
    HttpError, HttpOption, Method, RequestBody, RequestConstructions, RequestNoBody, RequestResult,
    Response,
};
use crate::transport::{self, Failure, Request};
use crate::{Component, HttpcProfile};
use geam::HostFailure;
use geam::gleam_erlang::{Charlist, service};
use geam::gleam_stdlib::provider_support::{Dynamic, DynamicSchema, GleamError, GleamOk};
use geam::host::{
    HostCall, HostCallCompletion, HostCallContinuation, HostCallError, HostConstructions,
    HostCustom, HostCustomConstructorAt, HostCustomIndex0, HostCustomIndexNext, HostExternal,
    HostList, HostListType, HostOwnedCompletion, HostProviderModule, HostRegistrationError,
    HostTuple,
};
use geam::provider::advanced::NativeValue;
use geam::provider::{BigInt, BitArrayValue, StringValue};
use std::sync::Arc;
use std::time::Duration;

type I0 = geam::host::HostTypeIndex0;
type I1 = geam::host::HostTypeIndexNext<I0>;
type I2 = geam::host::HostTypeIndexNext<I1>;
type I3 = geam::host::HostTypeIndexNext<I2>;
type I4 = geam::host::HostTypeIndexNext<I3>;
type I5 = geam::host::HostTypeIndexNext<I4>;
type I6 = geam::host::HostTypeIndexNext<I5>;
type C0 = HostCustomIndex0;
type C1 = HostCustomIndexNext<C0>;
type C2 = HostCustomIndexNext<C1>;
type C3 = HostCustomIndexNext<C2>;
type C4 = HostCustomIndexNext<C3>;
type C5 = HostCustomIndexNext<C4>;
type C6 = HostCustomIndexNext<C5>;
type C7 = HostCustomIndexNext<C6>;
type C8 = HostCustomIndexNext<C7>;
type C9 = HostCustomIndexNext<C8>;
type OtherMethod = HostCustomConstructorAt<Method, C9, schema::Other>;
type Ssl = HostCustomConstructorAt<HttpOption, C0, schema::Ssl>;
type Autoredirect = HostCustomConstructorAt<HttpOption, C1, schema::Autoredirect>;
type Timeout = HostCustomConstructorAt<HttpOption, C2, schema::Timeout>;
type ResponseTimeout = HostCustomConstructorAt<HttpError, C2, schema::ResponseTimeout>;
type FailedToConnect = HostCustomConstructorAt<HttpError, C1, schema::FailedToConnect>;
type Posix = HostCustomConstructorAt<ConnectError, C0, schema::Posix>;
type TlsAlert = HostCustomConstructorAt<ConnectError, C1, schema::TlsAlert>;

type Call<'call, Profile, Return> = HostCall<'call, Profile, Component<Profile>, Return>;

pub(crate) fn provider<Profile: HttpcProfile>()
-> Result<HostProviderModule<Profile>, HostRegistrationError> {
    HostProviderModule::new("gleam_httpc", "gleam/httpc")
        .and_then(|module| module.with_scoped_function_and_constructions::<Component<Profile>, (), Header, DefaultConstructions, _>(
            "default_user_agent", default_user_agent::<Profile>,
        ))
        .and_then(|module| module.with_scoped_function_and_constructions::<Component<Profile>, (Dynamic,), HttpError, ErrorConstructions, _>(
            "normalise_error", normalise_error::<Profile>,
        ))
        .and_then(|module| module.with_resumable_function::<Component<Profile>, (Method, RequestBody, HostListType<HttpOption>, HostListType<ErlOption>), RequestResult, RequestConstructions, _>(
            "erl_request", request_body::<Profile>,
        ))
        .and_then(|module| module.with_resumable_function::<Component<Profile>, (Method, RequestNoBody, HostListType<HttpOption>, HostListType<ErlOption>), RequestResult, RequestConstructions, _>(
            "erl_request_no_body", request_no_body::<Profile>,
        ))
}

fn charlist<'call, Profile: HttpcProfile, Return: geam::host::HostType>(
    call: &mut Call<'call, Profile, Return>,
    constructions: &HostConstructions<'call, RequestConstructions>,
    value: &str,
) -> HostExternal<'call, Charlist> {
    service::charlist_from_string(
        call,
        constructions.at::<I1>(),
        constructions.at::<I2>(),
        value,
    )
}

fn default_user_agent<'call, Profile: HttpcProfile>(
    mut call: Call<'call, Profile, Header>,
    constructions: HostConstructions<'call, DefaultConstructions>,
) -> Result<HostCallCompletion<'call, Header>, HostCallError> {
    let name = service::charlist_from_string(
        &mut call,
        constructions.at::<I0>(),
        constructions.at::<I1>(),
        "user-agent",
    );
    let value = service::charlist_from_string(
        &mut call,
        constructions.at::<I0>(),
        constructions.at::<I1>(),
        "gleam_httpc/5.0.0",
    );
    Ok(call.return_tuple((name, (value, ()))))
}

fn normalise_error<'call, Profile: HttpcProfile>(
    mut call: Call<'call, Profile, HttpError>,
    constructions: HostConstructions<'call, ErrorConstructions>,
    error: HostExternal<'call, Dynamic>,
) -> Result<HostCallCompletion<'call, HttpError>, HostCallError> {
    parse_error(call.native_value::<Dynamic>(error)).map(|native| match native {
        NativeHttpError::Timeout => call.return_custom::<ResponseTimeout>(()),
        NativeHttpError::FailedToConnect { ip4, ip6 } => {
            let ip4 = construct_connect(&mut call, &constructions, ip4);
            let ip6 = construct_connect(&mut call, &constructions, ip6);
            call.return_custom::<FailedToConnect>((ip4, (ip6, ())))
        }
    })
}

#[derive(Debug, PartialEq, Eq)]
enum NativeHttpError {
    Timeout,
    FailedToConnect {
        ip4: transport::ConnectError,
        ip6: transport::ConnectError,
    },
}

fn parse_error(native: NativeValue) -> Result<NativeHttpError, HostCallError> {
    if native.as_symbol().as_deref() == Some("timeout") {
        return Ok(NativeHttpError::Timeout);
    }
    if native
        .index(0)
        .and_then(|value| value.as_symbol())
        .as_deref()
        != Some("failed_connect")
    {
        return Err(HostFailure::new("unexpected gleam_httpc request error").into());
    }
    let ip4 = parse_connect(native.index(1))?;
    let ip6 = parse_connect(native.index(2))?;
    Ok(NativeHttpError::FailedToConnect { ip4, ip6 })
}

fn parse_connect(value: Option<NativeValue>) -> Result<transport::ConnectError, HostCallError> {
    let value = value.ok_or_else(|| HostFailure::new("missing gleam_httpc connection error"))?;
    let code = value
        .index(1)
        .and_then(|value| value.as_symbol())
        .ok_or_else(|| HostFailure::new("missing gleam_httpc connection error code"))?
        .to_string();
    match value
        .index(0)
        .and_then(|value| value.as_symbol())
        .as_deref()
    {
        Some("posix") => Ok(transport::ConnectError::Posix(code)),
        Some("tls_alert") => {
            let detail = value
                .index(2)
                .and_then(|value| value.as_symbol())
                .ok_or_else(|| HostFailure::new("missing gleam_httpc TLS alert detail"))?
                .to_string();
            Ok(transport::ConnectError::TlsAlert { code, detail })
        }
        _ => Err(HostFailure::new("unexpected gleam_httpc connection error").into()),
    }
}

fn construct_connect<'call, Profile: HttpcProfile>(
    call: &mut Call<'call, Profile, HttpError>,
    constructions: &HostConstructions<'call, ErrorConstructions>,
    value: transport::ConnectError,
) -> HostCustom<'call, ConnectError> {
    let token = constructions.at::<I0>();
    match value {
        transport::ConnectError::Posix(code) => {
            call.construct_custom::<Posix>(token, (StringValue::from(code), ()))
        }
        transport::ConnectError::TlsAlert { code, detail } => call.construct_custom::<TlsAlert>(
            token,
            (StringValue::from(code), (StringValue::from(detail), ())),
        ),
    }
}

fn method<'call, Profile: HttpcProfile>(
    call: &mut Call<'call, Profile, RequestResult>,
    value: HostCustom<'call, Method>,
) -> String {
    match call.custom_constructor(value) {
        0 => "GET".to_owned(),
        1 => "POST".to_owned(),
        2 => "HEAD".to_owned(),
        3 => "PUT".to_owned(),
        4 => "DELETE".to_owned(),
        5 => "TRACE".to_owned(),
        6 => "CONNECT".to_owned(),
        7 => "OPTIONS".to_owned(),
        8 => "PATCH".to_owned(),
        _ => {
            let (name, ()) = call.provider_borrow_remaining_custom_fields::<OtherMethod>(value);
            name.as_str().to_owned()
        }
    }
}

fn headers<'call, Profile: HttpcProfile>(
    call: &mut Call<'call, Profile, RequestResult>,
    headers: HostList<'call, Header>,
) -> Vec<(String, String)> {
    let mut output = Vec::with_capacity(call.list_len(headers));
    let mut index = 0;
    while let Some(header) = call.list_item::<Header>(headers, index) {
        let (name, (value, ())) = call.tuple_values(header);
        output.push((
            service::charlist_string(call, name).to_string(),
            service::charlist_string(call, value).to_string(),
        ));
        index += 1;
    }
    output
}

fn options<'call, Profile: HttpcProfile>(
    call: &mut Call<'call, Profile, RequestResult>,
    options: HostList<'call, HttpOption>,
) -> Result<(bool, bool, Duration), HostCallError> {
    let mut verify_tls = true;
    let mut follow_redirects = false;
    let mut timeout = 30_000_u64;
    let mut index = 0;
    while let Some(option) = call.list_item::<HttpOption>(options, index) {
        match call.custom_constructor(option) {
            0 => {
                let (_options, ()) = call.provider_borrow_remaining_custom_fields::<Ssl>(option);
                verify_tls = false;
            }
            1 => {
                let (enabled, ()) =
                    call.provider_borrow_remaining_custom_fields::<Autoredirect>(option);
                follow_redirects = enabled;
            }
            _ => {
                let (millis, ()) = call.provider_borrow_remaining_custom_fields::<Timeout>(option);
                timeout = millis.to_string().parse().map_err(|_| {
                    HostFailure::new("gleam_httpc timeout must be a non-negative millisecond value")
                })?;
            }
        }
        index += 1;
    }
    Ok((verify_tls, follow_redirects, Duration::from_millis(timeout)))
}

fn request_body<'call, Profile: HttpcProfile>(
    mut call: Call<'call, Profile, RequestResult>,
    constructions: HostConstructions<'call, RequestConstructions>,
    method_value: HostCustom<'call, Method>,
    request_value: HostTuple<'call, schema::Four<Charlist, Headers, Charlist, BitArrayValue>>,
    http_options: HostList<'call, HttpOption>,
    _options: HostList<'call, ErlOption>,
) -> Result<HostCallContinuation<'call, RequestResult>, HostCallError> {
    let method = method(&mut call, method_value);
    let (url, (request_headers, (content_type, (body, ())))) = call.tuple_values(request_value);
    let url = service::charlist_string(&mut call, url).to_string();
    let content_type = service::charlist_string(&mut call, content_type).to_string();
    let headers = headers(&mut call, request_headers);
    if !body.bit_len().is_multiple_of(8) {
        return Err(HostFailure::new("gleam_httpc request body must be byte aligned").into());
    }
    let body = body.bytes().to_vec();
    let (verify_tls, follow_redirects, timeout) = options(&mut call, http_options)?;
    continue_request(
        call,
        constructions,
        Request {
            method,
            url,
            headers,
            content_type: Some(content_type),
            body: Some(body),
            verify_tls,
            follow_redirects,
            timeout,
        },
    )
}

fn request_no_body<'call, Profile: HttpcProfile>(
    mut call: Call<'call, Profile, RequestResult>,
    constructions: HostConstructions<'call, RequestConstructions>,
    method_value: HostCustom<'call, Method>,
    request_value: HostTuple<'call, schema::Two<Charlist, Headers>>,
    http_options: HostList<'call, HttpOption>,
    _options: HostList<'call, ErlOption>,
) -> Result<HostCallContinuation<'call, RequestResult>, HostCallError> {
    let method = method(&mut call, method_value);
    let (url, (request_headers, ())) = call.tuple_values(request_value);
    let url = service::charlist_string(&mut call, url).to_string();
    let headers = headers(&mut call, request_headers);
    let (verify_tls, follow_redirects, timeout) = options(&mut call, http_options)?;
    continue_request(
        call,
        constructions,
        Request {
            method,
            url,
            headers,
            content_type: None,
            body: None,
            verify_tls,
            follow_redirects,
            timeout,
        },
    )
}

fn continue_request<'call, Profile: HttpcProfile>(
    mut call: Call<'call, Profile, RequestResult>,
    constructions: HostConstructions<'call, RequestConstructions>,
    request: Request,
) -> Result<HostCallContinuation<'call, RequestResult>, HostCallError> {
    let transport: Arc<dyn transport::Transport> = Arc::clone(&call.state().transport);
    Ok(call.resume(constructions, move |_context| {
        Box::pin(async move {
            let result = transport.send(request).await;
            Ok(HostOwnedCompletion::new(move |call, constructions| {
                complete_request(call, constructions, result)
            }))
        })
    }))
}

fn complete_request<'call, Profile: HttpcProfile>(
    mut call: Call<'call, Profile, RequestResult>,
    constructions: HostConstructions<'call, RequestConstructions>,
    result: Result<transport::Response, Failure>,
) -> Result<HostCallCompletion<'call, RequestResult>, HostCallError> {
    match result {
        Ok(response) => {
            let version = charlist(&mut call, &constructions, &response.version);
            let reason = charlist(&mut call, &constructions, &response.reason);
            let status = call.construct_tuple(
                constructions.at::<I3>(),
                (version, (BigInt::from(response.status), (reason, ()))),
            );
            let mut headers = Vec::with_capacity(response.headers.len());
            for (name, value) in response.headers {
                let name = charlist(&mut call, &constructions, &name);
                let value = charlist(&mut call, &constructions, &value);
                headers.push(call.construct_tuple(constructions.at::<I4>(), (name, (value, ()))));
            }
            let headers = call.construct_list(constructions.at::<I5>(), headers);
            let response = call.construct_tuple(
                constructions.at::<I6>(),
                (
                    status,
                    (headers, (BitArrayValue::from_bytes(response.body), ())),
                ),
            );
            Ok(call.return_custom::<GleamOk<Response, Dynamic>>((response, ())))
        }
        Err(Failure::InvalidRequest(reason)) => Err(HostFailure::new(reason).into()),
        Err(Failure::Timeout) => {
            complete_error(call, constructions, NativeValue::symbol("timeout"))
        }
        Err(Failure::FailedToConnect { ip4, ip6 }) => complete_error(
            call,
            constructions,
            NativeValue::tuple([
                NativeValue::symbol("failed_connect"),
                connect_native(ip4),
                connect_native(ip6),
            ]),
        ),
    }
}

fn complete_error<'call, Profile: HttpcProfile>(
    mut call: Call<'call, Profile, RequestResult>,
    constructions: HostConstructions<'call, RequestConstructions>,
    native: NativeValue,
) -> Result<HostCallCompletion<'call, RequestResult>, HostCallError> {
    let dynamic = call.construct_external_with_binding::<
        geam::gleam_erlang::Component<Profile>, DynamicSchema, geam::host::HostTypeListEnd,
    >(
        constructions.at::<I0>(),
        geam::gleam_stdlib::Dynamic::from_native(native),
    );
    Ok(call.return_custom::<GleamError<Response, Dynamic>>((dynamic, ())))
}

fn connect_native(value: transport::ConnectError) -> NativeValue {
    match value {
        transport::ConnectError::Posix(code) => {
            NativeValue::tuple([NativeValue::symbol("posix"), NativeValue::symbol(code)])
        }
        transport::ConnectError::TlsAlert { code, detail } => NativeValue::tuple([
            NativeValue::symbol("tls_alert"),
            NativeValue::symbol(code),
            NativeValue::symbol(detail),
        ]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geam::host::{
        HostComponentProfile, HostProviderComponent, HostProviderComponentInitialization,
    };
    use geam::{HostProfile, HostProviderSet, HostedExecution, ModuleSource, PackageSource};

    #[test]
    fn native_error_parser_rejects_unrecognized_shapes() {
        assert_eq!(
            parse_error(NativeValue::symbol("timeout")).unwrap(),
            NativeHttpError::Timeout
        );
        for malformed in [
            NativeValue::symbol("unexpected"),
            NativeValue::tuple([NativeValue::symbol("failed_connect")]),
            NativeValue::tuple([
                NativeValue::symbol("failed_connect"),
                NativeValue::tuple([NativeValue::symbol("posix")]),
                connect_native(transport::ConnectError::Posix("eio".into())),
            ]),
            NativeValue::tuple([
                NativeValue::symbol("failed_connect"),
                NativeValue::tuple([NativeValue::symbol("unknown"), NativeValue::symbol("eio")]),
                connect_native(transport::ConnectError::Posix("eio".into())),
            ]),
            NativeValue::tuple([
                NativeValue::symbol("failed_connect"),
                NativeValue::tuple([
                    NativeValue::symbol("tls_alert"),
                    NativeValue::symbol("unknown_ca"),
                ]),
                connect_native(transport::ConnectError::Posix("eio".into())),
            ]),
            NativeValue::tuple([
                NativeValue::symbol("failed_connect"),
                connect_native(transport::ConnectError::Posix("eio".into())),
                NativeValue::tuple([NativeValue::symbol("posix")]),
            ]),
        ] {
            assert!(parse_error(malformed).is_err());
        }
    }

    struct ErrorProfile;

    type Stdlib = geam::gleam_stdlib::Component<Vec<geam::gleam_stdlib::IoOutput>>;
    type Erlang = geam::gleam_erlang::Component<ErrorProfile>;
    type Httpc = Component<ErrorProfile>;

    #[derive(Default)]
    struct ErrorStores {
        stdlib: <Stdlib as HostProviderComponent>::Stores,
        erlang: <Erlang as HostProviderComponent>::Stores,
        httpc: <Httpc as HostProviderComponent>::Stores,
    }

    struct ErrorState {
        stdlib: geam::gleam_stdlib::GleamStdlibRunState<Vec<geam::gleam_stdlib::IoOutput>>,
        erlang: geam::gleam_erlang::Configuration,
        httpc: crate::State,
    }

    impl HostProfile for ErrorProfile {
        type RunState = ErrorState;
        type ExternalStores = ErrorStores;
        type ExecutionState =
            geam::execution::ExecutionServices<geam::gleam_erlang::ErlangExecution, ()>;

        fn initialize_execution(state: &mut Self::RunState) -> Self::ExecutionState {
            geam::execution::ExecutionServices {
                first: <Erlang as geam::HostExecutionService>::initialize_service(
                    &mut state.erlang,
                ),
                rest: (),
            }
        }
    }

    impl geam::gleam_stdlib::GleamStdlibHostProfile for ErrorProfile {
        type Io = Vec<geam::gleam_stdlib::IoOutput>;
    }

    impl HostComponentProfile<Stdlib> for ErrorProfile {
        fn component_stores(stores: &ErrorStores) -> &<Stdlib as HostProviderComponent>::Stores {
            &stores.stdlib
        }

        fn component_state(
            state: &mut ErrorState,
        ) -> &mut <Stdlib as HostProviderComponent>::RunState {
            &mut state.stdlib
        }
    }

    impl HostComponentProfile<Erlang> for ErrorProfile {
        fn component_stores(stores: &ErrorStores) -> &<Erlang as HostProviderComponent>::Stores {
            &stores.erlang
        }

        fn component_state(
            state: &mut ErrorState,
        ) -> &mut <Erlang as HostProviderComponent>::RunState {
            &mut state.erlang
        }
    }

    impl HostComponentProfile<Httpc> for ErrorProfile {
        fn component_stores(stores: &ErrorStores) -> &<Httpc as HostProviderComponent>::Stores {
            &stores.httpc
        }

        fn component_state(
            state: &mut ErrorState,
        ) -> &mut <Httpc as HostProviderComponent>::RunState {
            &mut state.httpc
        }
    }

    impl geam::gleam_erlang::GleamErlangHostProfile for ErrorProfile {
        fn erlang_execution(
            state: &mut Self::ExecutionState,
        ) -> &mut geam::gleam_erlang::ErlangExecution {
            &mut state.first
        }
    }

    fn malformed_dynamic<'call>(
        mut call: HostCall<'call, ErrorProfile, Erlang, Dynamic>,
    ) -> Result<HostCallCompletion<'call, Dynamic>, HostCallError> {
        let value = call.create_external(geam::gleam_stdlib::Dynamic::from_native(
            NativeValue::symbol("unexpected"),
        ));
        Ok(call.return_value(value))
    }

    fn timeout_dynamic<'call>(
        mut call: HostCall<'call, ErrorProfile, Erlang, Dynamic>,
    ) -> Result<HostCallCompletion<'call, Dynamic>, HostCallError> {
        let value = call.create_external(geam::gleam_stdlib::Dynamic::from_native(
            NativeValue::symbol("timeout"),
        ));
        Ok(call.return_value(value))
    }

    fn connection_dynamic<'call>(
        mut call: HostCall<'call, ErrorProfile, Erlang, Dynamic>,
    ) -> Result<HostCallCompletion<'call, Dynamic>, HostCallError> {
        let value = call.create_external(geam::gleam_stdlib::Dynamic::from_native(
            NativeValue::tuple([
                NativeValue::symbol("failed_connect"),
                connect_native(transport::ConnectError::Posix("eio".to_owned())),
                connect_native(transport::ConnectError::TlsAlert {
                    code: "unknown_ca".to_owned(),
                    detail: "certificate rejected".to_owned(),
                }),
            ]),
        ));
        Ok(call.return_value(value))
    }

    #[test]
    fn normalise_error_preserves_known_values_and_rejects_malformed_dynamic() {
        let dynamic = HostProviderModule::new("gleam_stdlib", "gleam/dynamic")
            .unwrap()
            .with_external_type::<Erlang, DynamicSchema>()
            .unwrap()
            .with_scoped_function::<Erlang, (), Dynamic, _>("malformed", malformed_dynamic)
            .unwrap()
            .with_scoped_function::<Erlang, (), Dynamic, _>("timeout", timeout_dynamic)
            .unwrap()
            .with_scoped_function::<Erlang, (), Dynamic, _>("connection", connection_dynamic)
            .unwrap();
        let httpc = HostProviderModule::new("gleam_httpc", "gleam/httpc")
            .unwrap()
            .with_scoped_function_and_constructions::<Httpc, (Dynamic,), HttpError, ErrorConstructions, _>(
                "normalise_error", normalise_error::<ErrorProfile>,
            )
            .unwrap();
        let providers = HostProviderSet::from_providers([dynamic, httpc]).unwrap();
        let packages = [
            PackageSource::new(
                "gleam_stdlib",
                Vec::<String>::new(),
                [ModuleSource::new(
                    "gleam/dynamic",
                    "dynamic.gleam",
                    "pub type Dynamic\n@external(erlang, \"fixture\", \"malformed\") pub fn malformed() -> Dynamic\n@external(erlang, \"fixture\", \"timeout\") pub fn timeout() -> Dynamic\n@external(erlang, \"fixture\", \"connection\") pub fn connection() -> Dynamic",
                )],
            ),
            PackageSource::new(
                "gleam_httpc",
                ["gleam_stdlib"],
                [ModuleSource::new(
                    "gleam/httpc",
                    "httpc.gleam",
                    r#"
import gleam/dynamic.{type Dynamic}
pub type ConnectError { Posix(code: String) TlsAlert(code: String, detail: String) }
pub type HttpError {
  InvalidUtf8Response
  FailedToConnect(ip4: ConnectError, ip6: ConnectError)
  ResponseTimeout
}
@external(erlang, "gleam_httpc_ffi", "normalise_error")
fn normalise_error(value: Dynamic) -> HttpError
pub fn probe(value: Dynamic) -> HttpError { normalise_error(value) }
"#,
                )],
            ),
            PackageSource::new(
                "application",
                ["gleam_stdlib", "gleam_httpc"],
                [ModuleSource::new(
                    "main",
                    "main.gleam",
                    r#"
import gleam/dynamic
import gleam/httpc
pub fn main() {
  let assert httpc.ResponseTimeout = httpc.probe(dynamic.timeout())
  let assert httpc.FailedToConnect(
    httpc.Posix("eio"),
    httpc.TlsAlert("unknown_ca", "certificate rejected"),
  ) = httpc.probe(dynamic.connection())
  httpc.probe(dynamic.malformed())
}
"#,
                )],
            ),
        ];
        let typed =
            geam::compile_typed_host_program("application", "main", packages, providers).unwrap();
        let mut execution =
            HostedExecution::try_from_module_plan(geam::plan_host_program(typed).unwrap()).unwrap();
        let executor = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let host = geam::execution::TokioHost::new(executor.handle().clone());
        let mut state = ErrorState {
            stdlib: geam::gleam_stdlib::GleamStdlibRunState::from_seed([0; 32]),
            erlang: geam::gleam_erlang::Configuration::default(),
            httpc: Httpc::initialize(&geam::HostProviderConfiguration::empty()).unwrap(),
        };
        let stores = ErrorStores::default();
        assert!(std::ptr::eq(
            <ErrorProfile as HostComponentProfile<Stdlib>>::component_stores(&stores),
            &raw const stores.stdlib,
        ));
        assert!(std::ptr::eq(
            <ErrorProfile as HostComponentProfile<Erlang>>::component_stores(&stores),
            &raw const stores.erlang,
        ));
        assert!(std::ptr::eq(
            <ErrorProfile as HostComponentProfile<Httpc>>::component_stores(&stores),
            &raw const stores.httpc,
        ));
        assert!(std::ptr::eq(
            <ErrorProfile as HostComponentProfile<Stdlib>>::component_state(&mut state),
            &raw const state.stdlib,
        ));
        assert!(std::ptr::eq(
            <ErrorProfile as HostComponentProfile<Erlang>>::component_state(&mut state),
            &raw const state.erlang,
        ));
        assert!(std::ptr::eq(
            <ErrorProfile as HostComponentProfile<Httpc>>::component_state(&mut state),
            &raw const state.httpc,
        ));
        let mut execution_state = ErrorProfile::initialize_execution(&mut state);
        assert!(std::ptr::eq(
            <ErrorProfile as geam::gleam_erlang::GleamErlangHostProfile>::erlang_execution(
                &mut execution_state,
            ),
            &raw const execution_state.first,
        ));
        let mut echo = Vec::new();
        let result = executor.block_on(execution.run_main(&host, &mut state, &mut echo));
        assert!(
            format!("{result:?}").contains("unexpected gleam_httpc request error"),
            "{result:?}"
        );
    }
}

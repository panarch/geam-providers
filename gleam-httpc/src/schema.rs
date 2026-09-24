//! Exact native shapes declared by the original `gleam_httpc` and `gleam_http` sources.

use geam::gleam_erlang::Charlist;
use geam::gleam_stdlib::provider_support::{Dynamic, GleamResult};
use geam::host::{
    HostCustomConstructorDefinition, HostCustomConstructorList, HostCustomConstructorListEnd,
    HostCustomField, HostCustomFieldList, HostCustomFieldListEnd, HostCustomSchema, HostCustomType,
    HostListType, HostTupleType, HostTypeList, HostTypeListEnd,
};
use geam::provider::{BigInt, BitArrayValue, StringValue};

macro_rules! one {
    ($field:ident : $type:ty) => {
        pub(crate) struct $field;
        impl HostCustomField for $field {
            const LABEL: Option<&'static str> = None;
            type Type = $type;
        }
    };
}

macro_rules! field {
    ($field:ident : $type:ty => $label:literal) => {
        pub(crate) struct $field;
        impl HostCustomField for $field {
            const LABEL: Option<&'static str> = Some($label);
            type Type = $type;
        }
    };
}

macro_rules! empty {
    ($name:ident => $source:literal) => {
        pub(crate) struct $name;
        impl HostCustomConstructorDefinition for $name {
            const NAME: &'static str = $source;
            type Fields = HostCustomFieldListEnd;
        }
    };
}

macro_rules! unary {
    ($name:ident => $source:literal ($field:ident : $type:ty)) => {
        one!($field : $type);
        pub(crate) struct $name;
        impl HostCustomConstructorDefinition for $name {
            const NAME: &'static str = $source;
            type Fields = HostCustomFieldList<$field, HostCustomFieldListEnd>;
        }
    };
}

macro_rules! constructors {
    () => { HostCustomConstructorListEnd };
    ($first:ident $(, $rest:ident)* $(,)?) => {
        HostCustomConstructorList<$first, constructors!($($rest),*)>
    };
}

macro_rules! schema {
    ($id:ident : $type:ident = $package:literal / $module:literal / $name:literal [$($constructor:ident),+]) => {
        pub(crate) struct $id;
        pub(crate) type $type = HostCustomType<$id>;
        impl HostCustomSchema for $id {
            const PACKAGE: &'static str = $package;
            const MODULE: &'static str = $module;
            const NAME: &'static str = $name;
            const PARAMETER_COUNT: usize = 0;
            type Constructors = constructors!($($constructor),+);
        }
    };
}

empty!(Get => "Get");
empty!(Post => "Post");
empty!(Head => "Head");
empty!(Put => "Put");
empty!(Delete => "Delete");
empty!(Trace => "Trace");
empty!(Connect => "Connect");
empty!(Options => "Options");
empty!(Patch => "Patch");
unary!(Other => "Other" (OtherValue: StringValue));
schema!(MethodSchema: Method = "gleam_http" / "gleam/http" / "Method" [
    Get, Post, Head, Put, Delete, Trace, Connect, Options, Patch, Other
]);

empty!(VerifyNone => "VerifyNone");
schema!(VerifyOptionSchema: VerifyOption = "gleam_httpc" / "gleam/httpc" / "ErlVerifyOption" [VerifyNone]);
unary!(Verify => "Verify" (VerifyValue: VerifyOption));
schema!(SslOptionSchema: SslOption = "gleam_httpc" / "gleam/httpc" / "ErlSslOption" [Verify]);
unary!(Ssl => "Ssl" (SslValue: HostListType<SslOption>));
unary!(Autoredirect => "Autoredirect" (AutoredirectValue: bool));
unary!(Timeout => "Timeout" (TimeoutValue: BigInt));
schema!(HttpOptionSchema: HttpOption = "gleam_httpc" / "gleam/httpc" / "ErlHttpOption" [
    Ssl, Autoredirect, Timeout
]);

empty!(Binary => "Binary");
schema!(BodyFormatSchema: BodyFormat = "gleam_httpc" / "gleam/httpc" / "BodyFormat" [Binary]);
empty!(Inet6fb4Constructor => "Inet6fb4");
schema!(Inet6fb4Schema: Inet6fb4 = "gleam_httpc" / "gleam/httpc" / "Inet6fb4" [Inet6fb4Constructor]);
unary!(Ipfamily => "Ipfamily" (IpfamilyValue: Inet6fb4));
schema!(SocketOptSchema: SocketOpt = "gleam_httpc" / "gleam/httpc" / "SocketOpt" [Ipfamily]);
unary!(BodyFormatOption => "BodyFormat" (BodyFormatValue: BodyFormat));
unary!(SocketOpts => "SocketOpts" (SocketOptsValue: HostListType<SocketOpt>));
schema!(ErlOptionSchema: ErlOption = "gleam_httpc" / "gleam/httpc" / "ErlOption" [BodyFormatOption, SocketOpts]);

empty!(InvalidUtf8Response => "InvalidUtf8Response");
field!(Ip4: ConnectError => "ip4");
field!(Ip6: ConnectError => "ip6");
pub(crate) struct FailedToConnect;
impl HostCustomConstructorDefinition for FailedToConnect {
    const NAME: &'static str = "FailedToConnect";
    type Fields = HostCustomFieldList<Ip4, HostCustomFieldList<Ip6, HostCustomFieldListEnd>>;
}
empty!(ResponseTimeout => "ResponseTimeout");
schema!(HttpErrorSchema: HttpError = "gleam_httpc" / "gleam/httpc" / "HttpError" [
    InvalidUtf8Response, FailedToConnect, ResponseTimeout
]);

field!(Code: StringValue => "code");
pub(crate) struct Posix;
impl HostCustomConstructorDefinition for Posix {
    const NAME: &'static str = "Posix";
    type Fields = HostCustomFieldList<Code, HostCustomFieldListEnd>;
}
field!(Detail: StringValue => "detail");
pub(crate) struct TlsAlert;
impl HostCustomConstructorDefinition for TlsAlert {
    const NAME: &'static str = "TlsAlert";
    type Fields = HostCustomFieldList<Code, HostCustomFieldList<Detail, HostCustomFieldListEnd>>;
}
schema!(ConnectErrorSchema: ConnectError = "gleam_httpc" / "gleam/httpc" / "ConnectError" [Posix, TlsAlert]);

pub(crate) type Header = HostTupleType<Two<Charlist, Charlist>>;
pub(crate) type Headers = HostListType<Header>;
pub(crate) type RequestNoBody = HostTupleType<Two<Charlist, Headers>>;
pub(crate) type RequestBody = HostTupleType<Four<Charlist, Headers, Charlist, BitArrayValue>>;
pub(crate) type Status = HostTupleType<Three<Charlist, BigInt, Charlist>>;
pub(crate) type Response = HostTupleType<Three<Status, Headers, BitArrayValue>>;
pub(crate) type RequestResult = GleamResult<Response, Dynamic>;
pub(crate) type DefaultConstructions = Two<Charlist, HostListType<char>>;
pub(crate) type ErrorConstructions = One<ConnectError>;
pub(crate) type RequestConstructions = HostTypeList<
    Dynamic,
    HostTypeList<
        Charlist,
        HostTypeList<
            HostListType<char>,
            HostTypeList<Status, HostTypeList<Header, HostTypeList<Headers, One<Response>>>>,
        >,
    >,
>;

pub(crate) type One<A> = HostTypeList<A, HostTypeListEnd>;
pub(crate) type Two<A, B> = HostTypeList<A, One<B>>;
pub(crate) type Three<A, B, C> = HostTypeList<A, Two<B, C>>;
pub(crate) type Four<A, B, C, D> = HostTypeList<A, Three<B, C, D>>;

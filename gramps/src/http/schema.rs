//! Exact nonopaque HTTP custom types declared in unchanged gramps 6.0.1.
use geam::gleam_stdlib::provider_support::{Dynamic, GleamResult};
use geam::host::{
    HostCustomConstructorAt, HostCustomConstructorDefinition, HostCustomConstructorList,
    HostCustomConstructorListEnd, HostCustomField, HostCustomFieldList, HostCustomFieldListEnd,
    HostCustomIndex0, HostCustomIndexNext, HostCustomSchema, HostCustomType, HostTupleType,
    HostTypeList, HostTypeListEnd,
};
use geam::provider::{BigInt, BitArrayValue, StringValue};
pub(super) type One<T> = HostTypeList<T, HostTypeListEnd>;
pub(super) type Two<A, B> = HostTypeList<A, One<B>>;
macro_rules! fields {
    () => { HostCustomFieldListEnd };
    ($head:ident $(, $tail:ident)*) => { HostCustomFieldList<$head, fields!($($tail),*)> };
}
macro_rules! constructors {
    () => { HostCustomConstructorListEnd };
    ($head:ident $(, $tail:ident)*) => { HostCustomConstructorList<$head, constructors!($($tail),*)> };
}
macro_rules! field {
    ($name:ident: $type:ty, $label:expr) => {
        pub struct $name;
        impl HostCustomField for $name {
            type Type = $type;
            const LABEL: Option<&'static str> = $label;
        }
    };
}
macro_rules! constructor {
    ($name:ident => $gleam:literal [$($field:ident),*]) => {
        pub struct $name;
        impl HostCustomConstructorDefinition for $name { const NAME: &'static str=$gleam; type Fields=fields!($($field),*); }
    };
}
macro_rules! schema {
    ($id:ident: $alias:ident = $package:literal / $module:literal / $name:literal [$($constructor:ident),+]) => {
        pub struct $id;
        pub type $alias=HostCustomType<$id>;
        impl HostCustomSchema for $id {
            const PACKAGE: &'static str=$package; const MODULE: &'static str=$module; const NAME: &'static str=$name;
            const PARAMETER_COUNT: usize=0; type Constructors=constructors!($($constructor),+);
        }
    };
}
constructor!(Http => "Http" []);
constructor!(Https => "Https" []);
schema!(SchemeSchema: Scheme = "gleam_http" / "gleam/http" / "Scheme" [Http,Https]);
constructor!(HttpBin => "HttpBin" []);
constructor!(HttphBin => "HttphBin" []);
schema!(PacketSchema: PacketType = "gramps" / "gramps/http" / "PacketType" [HttphBin,HttpBin]);
field!(Length: BigInt, Some("length"));
field!(Reason: StringValue, Some("reason"));
constructor!(More => "More" [Length]);
constructor!(HttpError => "HttpError" [Reason]);
schema!(ErrorSchema: DecodeError = "gramps" / "gramps/http" / "DecodePacketError" [More,HttpError]);
field!(Path: StringValue, None);
field!(UriScheme: Scheme, Some("scheme"));
field!(UriHost: StringValue, Some("host"));
field!(UriPort: BigInt, Some("port"));
field!(UriPath: StringValue, Some("path"));
constructor!(AbsPath => "AbsPath" [Path]);
constructor!(AbsoluteUri => "AbsoluteUri" [UriScheme,UriHost,UriPort,UriPath]);
schema!(UriSchema: Uri = "gramps" / "gramps/http" / "UriPacket" [AbsPath,AbsoluteUri]);
pub(super) type Version = HostTupleType<Two<BigInt, BigInt>>;
field!(Method: StringValue, Some("method"));
field!(RequestUri: Uri, Some("uri"));
field!(HttpVersion: Version, Some("version"));
field!(Status: BigInt, Some("status"));
field!(Text: StringValue, Some("text"));
field!(Unknown: BigInt, Some("unknown"));
field!(HeaderField: Dynamic, Some("field"));
field!(RawField: StringValue, Some("raw_field"));
field!(Value: StringValue, Some("value"));
constructor!(HttpRequest => "HttpRequest" [Method,RequestUri,HttpVersion]);
constructor!(HttpResponse => "HttpResponse" [HttpVersion,Status,Text]);
constructor!(HttpHeader => "HttpHeader" [Unknown,HeaderField,RawField,Value]);
constructor!(HttpEoh => "HttpEoh" []);
schema!(PacketResultSchema: DecodedPacket = "gramps" / "gramps/http" / "DecodedPacket" [HttpRequest,HttpResponse,HttpHeader,HttpEoh]);
pub type DecodeResult = GleamResult<HostTupleType<Two<DecodedPacket, BitArrayValue>>, DecodeError>;

type C0 = HostCustomIndex0;
type C1 = HostCustomIndexNext<C0>;
type C2 = HostCustomIndexNext<C1>;
type C3 = HostCustomIndexNext<C2>;
pub(super) type HeaderKind = HostCustomConstructorAt<PacketType, C0, HttphBin>;
pub(super) type HttpScheme = HostCustomConstructorAt<Scheme, C0, Http>;
pub(super) type HttpsScheme = HostCustomConstructorAt<Scheme, C1, Https>;
pub(super) type OriginUri = HostCustomConstructorAt<Uri, C0, AbsPath>;
pub(super) type Absolute = HostCustomConstructorAt<Uri, C1, AbsoluteUri>;
pub(super) type Request = HostCustomConstructorAt<DecodedPacket, C0, HttpRequest>;
pub(super) type Response = HostCustomConstructorAt<DecodedPacket, C1, HttpResponse>;
pub(super) type Header = HostCustomConstructorAt<DecodedPacket, C2, HttpHeader>;
pub(super) type Eoh = HostCustomConstructorAt<DecodedPacket, C3, HttpEoh>;
pub(super) type Incomplete = HostCustomConstructorAt<DecodeError, C0, More>;
pub(super) type Invalid = HostCustomConstructorAt<DecodeError, C1, HttpError>;

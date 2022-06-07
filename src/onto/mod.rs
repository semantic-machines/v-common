pub mod cbor2individual;
pub mod datatype;
pub mod individual;
pub mod individual2json;
pub mod individual2msgpack;
pub mod individual2turtle;
pub mod json2individual;
pub mod msgpack2individual;
pub mod onto_impl;
pub mod onto_index;
pub mod parser;
pub mod resource;
mod turtle_formatters_with_prefixes;

/// -9223372036854775808…+9223372036854775807 (64 bit).
pub const XSD_LONG: &str = "http://www.w3.org/2001/XMLSchema#long";
/// -2147483648…+2147483647 (32 bit).
pub const XSD_INT: &str = "http://www.w3.org/2001/XMLSchema#int";
/// Arbitrary-size integer numbers.
pub const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
/// true, false.
pub const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";
/// Arbitrary-precision decimal numbers.
pub const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";
/// 32-bit floating point numbers incl. ±Inf, ±0, NaN.
pub const XSD_FLOAT: &str = "http://www.w3.org/2001/XMLSchema#float";
/// 64-bit floating point numbers incl. ±Inf, ±0, NaN.
pub const XSD_DOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";
/// Date and time with required timezone.
pub const XSD_DATE_TIME: &str = "http://www.w3.org/2001/XMLSchema#dateTime";
/// Character strings (but not all Unicode character strings).
pub const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
/// Whitespace-normalized strings.
pub const XSD_NORMALIZED_STRING: &str = "http://www.w3.org/2001/XMLSchema#normalizedString";
/// Integer numbers <0.
pub const XSD_NEGATIVE_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#negativeInteger";
/// Integer numbers ≥0.
pub const XSD_NON_NEGATIVE_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#nonNegativeInteger";
/// Integer numbers ≤0.
pub const XSD_NON_POSITIVE_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#nonPositiveInteger";
/// Integer numbers >0.
pub const XSD_POSITIVE_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#positiveInteger";

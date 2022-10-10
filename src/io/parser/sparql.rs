//! Parsers for productions from the SPARQL 1.1 grammar.
use std::fmt::Display;

use macros::traced;
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{one_of, satisfy},
    combinator::{map, opt, recognize},
    multi::{many0, many1, separated_list0},
    sequence::{delimited, pair, preceded, terminated, tuple},
};

use super::{iri, rfc5234::digit, turtle::hex, types::IntermediateResult};

#[derive(Debug)]
pub enum Name<'a> {
    IriReference(&'a str),
    PrefixedName { prefix: &'a str, local: &'a str },
    BlankNode(&'a str),
}

impl<'a> Name<'a> {
    pub fn as_iri_reference(&'a self) -> Option<&'a str> {
        match *self {
            Name::IriReference(iri) => Some(iri),
            _ => None,
        }
    }

    pub fn as_blank_node_label(&'a self) -> Option<&'a str> {
        match *self {
            Name::BlankNode(label) => Some(label),
            _ => None,
        }
    }
}

impl Display for Name<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Name::IriReference(iri) => write!(f, "{iri}"),
            Name::PrefixedName { prefix, local } => write!(f, "{prefix}:{local}"),
            Name::BlankNode(label) => write!(f, "_:{label}"),
        }
    }
}

/// Parse an IRI reference, i.e., an IRI (relative or absolute)
/// wrapped in angle brackets. Roughly equivalent to the
/// [IRIREF](https://www.w3.org/TR/sparql11-query/#rIRIREF)
/// production of the SPARQL 1.1 grammar, but uses the full [RFC
/// 3987](https://www.ietf.org/rfc/rfc3987.txt) grammar to verify
/// the actual IRI.
#[traced("parser::sparql")]
pub fn iriref<'a>(input: &'a str) -> IntermediateResult<&'a str> {
    delimited(tag("<"), iri::iri_reference, tag(">"))(input)
}

#[traced("parser::sparql")]
pub fn iri(input: &str) -> IntermediateResult<Name> {
    alt((map(iriref, Name::IriReference), prefixed_name))(input)
}

#[traced("parser::sparql")]
pub fn pname_ns(input: &str) -> IntermediateResult<&str> {
    let (rest, prefix) = terminated(opt(pn_prefix), tag(":"))(input)?;

    Ok((rest, prefix.unwrap_or_default()))
}

#[traced("parser::sparql")]
pub fn pn_chars_base(input: &str) -> IntermediateResult<&str> {
    recognize(satisfy(|c| {
        [
            0x41_u32..=0x5A,
            0x61..=0x7A,
            0x00C0..=0x0D6,
            0x0D8..=0x0F6,
            0x00F8..=0x2FF,
            0x0370..=0x037D,
            0x037F..=0x1FFF,
            0x200C..=0x200D,
            0x2070..=0x218F,
            0x2C00..=0x2FEF,
            0x3001..=0xD7FF,
            0xF900..=0xFDCF,
            0xFDF0..=0xFFFD,
            0x10000..=0xEFFFF,
        ]
        .iter()
        .any(|range| range.contains(&c.into()))
    }))(input)
}

#[traced("parser::sparql")]
pub fn pn_chars_u(input: &str) -> IntermediateResult<&str> {
    alt((pn_chars_base, tag("_")))(input)
}

#[traced("parser::sparql")]
pub fn pn_chars(input: &str) -> IntermediateResult<&str> {
    alt((
        pn_chars_u,
        tag("-"),
        digit,
        tag("\u{00B7}"),
        recognize(satisfy(|c| {
            [0x0300u32..=0x036F, 0x203F..=0x2040]
                .iter()
                .any(|range| range.contains(&c.into()))
        })),
    ))(input)
}

#[traced("parser::sparql")]
pub fn pn_prefix(input: &str) -> IntermediateResult<&str> {
    recognize(tuple((
        pn_chars_base,
        separated_list0(many1(tag(".")), many0(pn_chars)),
    )))(input)
}

#[traced("parser::sparql")]
pub fn percent(input: &str) -> IntermediateResult<&str> {
    recognize(tuple((tag("%"), hex, hex)))(input)
}

#[traced("parser::sparql")]
pub fn pn_local_esc(input: &str) -> IntermediateResult<&str> {
    recognize(preceded(tag(r#"\"#), one_of(r#"_~.-!$&'()*+,;=/?#@%"#)))(input)
}

#[traced("parser::sparql")]
pub fn plx(input: &str) -> IntermediateResult<&str> {
    alt((percent, pn_local_esc))(input)
}

#[traced("parser::sparql")]
pub fn pn_local(input: &str) -> IntermediateResult<&str> {
    recognize(pair(
        alt((pn_chars_u, tag(":"), digit, plx)),
        opt(separated_list0(
            many1(tag(".")),
            many0(alt((pn_chars, tag(":"), plx))),
        )),
    ))(input)
}

#[traced("parser::sparql")]
pub fn pname_ln(input: &str) -> IntermediateResult<Name> {
    map(pair(pname_ns, pn_local), |(prefix, local)| {
        Name::PrefixedName { prefix, local }
    })(input)
}

#[traced("parser::sparql")]
pub fn prefixed_name(input: &str) -> IntermediateResult<Name> {
    alt((
        pname_ln,
        map(pname_ns, |prefix| Name::PrefixedName { prefix, local: "" }),
    ))(input)
}

#[traced("parser::sparql")]
pub fn blank_node_label(input: &str) -> IntermediateResult<Name> {
    preceded(
        tag("_:"),
        map(
            recognize(pair(
                alt((pn_chars_u, digit)),
                opt(separated_list0(many1(tag(".")), many0(pn_chars))),
            )),
            Name::BlankNode,
        ),
    )(input)
}

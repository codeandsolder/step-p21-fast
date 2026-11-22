use crate::{
    ast::{Anchor, AnchorItem},
    parser::{
        combinator::{
            ParseResult, char_, many0_, parenthesized_list, tag_, tuple_,
        },
        token::{
            anchor_name, enumeration, number, resource, rhs_occurrence_name,
            string, tag_name,
        },
    },
};
use nom::{
    Parser,
    error::{ErrorKind, ParseError},
};

/// `anchor_section` = `ANCHOR;` [`anchor_list`] `ENDSEC;` .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn anchor_section(input: &str) -> ParseResult<'_, Vec<Anchor>> {
    tuple_((tag_("ANCHOR;"), anchor_list, tag_("ENDSEC;")))
        .map(|(_start, anchors, _end)| anchors)
        .parse(input)
}

/// `anchor_list` = { [`anchor()`] } .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn anchor_list(input: &str) -> ParseResult<'_, Vec<Anchor>> {
    many0_(anchor).parse(input)
}

/// anchor = [`anchor_name`] `=` [`anchor_item`] { [`anchor_tag`] } `;` .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn anchor(input: &str) -> ParseResult<'_, Anchor> {
    tuple_((
        anchor_name,
        char_('='),
        anchor_item,
        many0_(anchor_tag),
        char_(';'),
    ))
    .map(|(name, _eq, item, tags, _semicolon)| Anchor { name, item, tags })
    .parse(input)
}

/// `anchor_item` = `$` | [`integer`](crate::parser::token::integer) |
/// [`real`](crate::parser::token::real) | [string] | [enumeration] | binary |
/// [`rhs_occurrence_name`] | [`crate::parser::token::resource`] |
/// [`anchor_item_list`] .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn anchor_item(input: &str) -> ParseResult<'_, AnchorItem> {
    match input.as_bytes() {
        [b'$', ..] => Ok((&input[1..], AnchorItem::NotProvided)),
        [b'0'..=b'9' | b'+' | b'-' | b' ' | b'\t' | b'\r' | b'\n', ..] => {
            number(input)
        }
        [b'\'', ..] => {
            string(input).map(|(rest, value)| (rest, AnchorItem::String(value)))
        }
        [b'#' | b'@', ..] => rhs_occurrence_name(input)
            .map(|(rest, value)| (rest, AnchorItem::Name(value))),
        [b'.', ..] => enumeration(input)
            .map(|(rest, value)| (rest, AnchorItem::Enumeration(value))),
        [b'<', ..] => resource(input)
            .map(|(rest, value)| (rest, AnchorItem::Resource(value))),
        [b'(', ..] => anchor_item_list(input),
        // Binary literals are not yet supported.
        _ => Err(nom::Err::Error(ParseError::from_error_kind(
            input,
            ErrorKind::Alt,
        ))),
    }
}

/// `anchor_item_list` = `(` \[ [`anchor_item`] { `,` [`anchor_item`] } \] `)` .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn anchor_item_list(input: &str) -> ParseResult<'_, AnchorItem> {
    parenthesized_list(anchor_item)
        .map(AnchorItem::List)
        .parse(input)
}

/// `anchor_tag` = `{` [`tag_name`] `:` [`anchor_item`] `}` .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn anchor_tag(input: &str) -> ParseResult<'_, (String, AnchorItem)> {
    tuple_((char_('{'), tag_name, char_(':'), anchor_item, char_('}')))
        .map(|(_open, name, _colon, item, _close)| (name, item))
        .parse(input)
}

#[cfg(test)]
mod tests {
    use crate::ast::{Anchor, AnchorItem, Name, ReferenceEntry, Uri};

    #[test]
    fn anchors_accept_real_numbers_resources_and_nested_lists() {
        let input = "ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('x'),'4;1');
FILE_NAME('x','',(),(),'','','');
FILE_SCHEMA(('X'));
ENDSEC;
ANCHOR;
<number> = -1.25E-2;
<data> = (1, -0.0, <https://example.invalid/model.step#part>,
          (#12, @13, .T., $, 'é', (/* empty */)))
          {label: <other.step#part>};
ENDSEC;
REFERENCE;
#12=<other.step#entity>;
ENDSEC;
DATA;
ENDSEC;
END-ISO-10303-21;";
        let exchange = crate::parser::parse(input).unwrap();
        assert_eq!(
            exchange.anchor,
            vec![
                Anchor {
                    name: "number".to_owned(),
                    item: AnchorItem::Real(-0.0125),
                    tags: Vec::new()
                },
                Anchor {
                    name: "data".to_owned(),
                    item: AnchorItem::List(vec![
                        AnchorItem::Integer(1),
                        AnchorItem::Real(-0.0),
                        AnchorItem::Resource(Uri(
                            "https://example.invalid/model.step#part"
                                .to_owned()
                        )),
                        AnchorItem::List(vec![
                            AnchorItem::Name(Name::Entity(12)),
                            AnchorItem::Name(Name::Value(13)),
                            AnchorItem::Enumeration("T".to_owned()),
                            AnchorItem::NotProvided,
                            AnchorItem::String("é".to_owned()),
                            AnchorItem::List(Vec::new()),
                        ]),
                    ]),
                    tags: vec![(
                        "label".to_owned(),
                        AnchorItem::Resource(Uri("other.step#part".to_owned()))
                    )]
                },
            ]
        );
        let AnchorItem::List(items) = &exchange.anchor[1].item else {
            panic!("expected list")
        };
        let AnchorItem::Real(value) = items[1] else {
            panic!("expected real")
        };
        assert_eq!(value.to_bits(), (-0.0_f64).to_bits());
        assert_eq!(
            exchange.reference,
            vec![ReferenceEntry {
                name: Name::Entity(12),
                resource: Uri("other.step#entity".to_owned()),
            }]
        );
    }

    #[test]
    fn anchor_items_preserve_numeric_residuals_and_reject_invalid_syntax() {
        assert_eq!(
            super::anchor_item("1.23E+2;"),
            Ok((";", AnchorItem::Real(123.0)))
        );
        assert_eq!(
            super::anchor_item("-9223372036854775808;"),
            Ok((";", AnchorItem::Integer(i64::MIN)))
        );
        assert_eq!(
            super::anchor_item("1.0E+;"),
            Ok(("E+;", AnchorItem::Real(1.0)))
        );
        for input in [
            "1.0E309",
            "(1.0,)",
            "(*, 1)",
            "(TYPED(1))",
            "(<unclosed)",
            "#18446744073709551616",
        ] {
            assert!(super::anchor_item(input).is_err(), "{input:?}");
        }
        assert_eq!(
            super::anchor_item("(<part>,(/*empty*/))tail"),
            Ok((
                "tail",
                AnchorItem::List(vec![
                    AnchorItem::Resource(Uri("part".to_owned())),
                    AnchorItem::List(Vec::new()),
                ])
            ))
        );
    }
}

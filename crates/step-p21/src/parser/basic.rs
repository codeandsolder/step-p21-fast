//! Parser for basic alphabets defined in the table 1 of ISO-10303-21

use super::combinator::ParseResult;
use nom::{
    Parser,
    branch::alt,
    character::complete::{char, satisfy},
};

/// `latin_codepoint` = [space] | [digit] | [lower] | [upper] | [special] |
/// [`reverse_solidus`] | [apostrophe]
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn latin_codepoint(input: &str) -> ParseResult<'_, char> {
    alt((
        space,
        digit,
        lower,
        upper,
        special,
        reverse_solidus,
        apostrophe,
    ))
    .parse(input)
}

/// space = ` ` .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn space(input: &str) -> ParseResult<'_, char> {
    char(' ')(input)
}

/// digit = `0` | `1` | `2` | `3` | `4` | `5` | `6` | `7` | `8` | `9` .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn digit(input: &str) -> ParseResult<'_, char> {
    satisfy(|c| c.is_ascii_digit()).parse(input)
}

/// lower = `a` | `b` | `c` | `d` | `e` | `f` | `g` | `h`
///       | `i` | `j` | `k` | `l` | `m` | `n` | `o` | `p`
///       | `q` | `r` | `s` | `t` | `u` | `v` | `w` | `x`
///       | `y` | `z` .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn lower(input: &str) -> ParseResult<'_, char> {
    satisfy(|c| c.is_ascii_lowercase()).parse(input)
}

/// upper = `A` | `B` | `C` | `D` | `E` | `F` | `G` | `H`
///       | `I` | `J` | `K` | `L` | `M` | `N` | `O` | `P`
///       | `Q` | `R` | `S` | `T` | `U` | `V` | `W` | `X`
///       | `Y` | `Z` | `_` .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn upper(input: &str) -> ParseResult<'_, char> {
    satisfy(|c| c.is_ascii_uppercase() || c == '_').parse(input)
}

/// special  = `!` | `"` | `*` | `$` | `%` | `&` | `.` | `#` | `+` | `,`  | `-`
/// | `(` | `)` | `?` | `/` | `:` | `;` | `<`  | `=` | `>` | `@` | `[` | `]` |
/// `{` | `|` | `}`  | `^` | \` | `~` .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn special(input: &str) -> ParseResult<'_, char> {
    satisfy(|c| {
        matches!(
            c,
            '!' | '"'
                | '*'
                | '$'
                | '%'
                | '&'
                | '.'
                | '#'
                | '+'
                | ','
                | '-'
                | '('
                | ')'
                | '?'
                | '/'
                | ':'
                | ';'
                | '<'
                | '='
                | '>'
                | '@'
                | '['
                | ']'
                | '{'
                | '|'
                | '}'
                | '^'
                | '`'
                | '~'
        )
    })
    .parse(input)
}

/// `reverse_solidus` = `\\` .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn reverse_solidus(input: &str) -> ParseResult<'_, char> {
    char('\\')(input)
}

/// apostrophe = `'` .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn apostrophe(input: &str) -> ParseResult<'_, char> {
    char('\'')(input)
}

use std::borrow::Cow;

use itertools::Itertools;
use winnow::combinator::*;
use winnow::prelude::*;
use winnow::token::*;

pub fn sq_dequote_step<'i>(input: &mut &'i str) -> PResult<Cow<'i, str>, ()> {
    // See git's quote.c's `sq_dequote_step`
    alt((sq_dequote_escaped, sq_dequote_no_escaped)).parse_next(input)
}

fn sq_dequote_escaped<'i>(input: &mut &'i str) -> PResult<Cow<'i, str>, ()> {
    (
        sq_dequote_section,
        sq_dequote_trail,
        repeat(0.., sq_dequote_trail),
    )
        .map(|(start, trail, mut trails): (_, _, Vec<_>)| {
            trails.insert(0, trail);
            trails.insert(0, [start, ""]);
            let value = trails.into_iter().flatten().join("");
            Cow::Owned(value)
        })
        .parse_next(input)
}

fn sq_dequote_no_escaped<'i>(input: &mut &'i str) -> PResult<Cow<'i, str>, ()> {
    sq_dequote_section.map(Cow::Borrowed).parse_next(input)
}

fn sq_dequote_section<'i>(input: &mut &'i str) -> PResult<&'i str, ()> {
    terminated(preceded('\'', take_while(0.., |c| c != '\'')), '\'').parse_next(input)
}

fn sq_dequote_trail<'i>(input: &mut &'i str) -> PResult<[&'i str; 2], ()> {
    (escaped, sq_dequote_section)
        .map(|(e, s)| [e, s])
        .parse_next(input)
}

fn escaped<'i>(input: &mut &'i str) -> PResult<&'i str, ()> {
    preceded('\\', one_of(['\'', '!']).recognize()).parse_next(input)
}

#[cfg(test)]
mod test_sq_dequote_step {
    use super::*;

    #[test]
    fn word() {
        let fixture = "'name'";
        let expected = Cow::Borrowed("name");
        let (_, actual) = sq_dequote_step.parse_peek(fixture).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn space() {
        let fixture = "'a b'";
        let expected = Cow::Borrowed("a b");
        let (_, actual) = sq_dequote_step.parse_peek(fixture).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn sq_escaped() {
        let fixture = "'a'\\''b'";
        let expected: Cow<str> = Cow::Owned("a'b".into());
        let (_, actual) = sq_dequote_step.parse_peek(fixture).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn exclamation_escaped() {
        let fixture = "'a'\\!'b'";
        let expected: Cow<str> = Cow::Owned("a!b".into());
        let (_, actual) = sq_dequote_step.parse_peek(fixture).unwrap();
        assert_eq!(actual, expected);
    }
}

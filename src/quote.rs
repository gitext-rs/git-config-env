use std::borrow::Cow;

use itertools::Itertools;
use winnow::branch::*;
use winnow::bytes::*;
use winnow::multi::*;
use winnow::prelude::*;
use winnow::sequence::*;

pub fn sq_dequote_step(input: &str) -> IResult<&str, Cow<str>> {
    // See git's quote.c's `sq_dequote_step`
    alt((sq_dequote_escaped, sq_dequote_no_escaped))(input)
}

fn sq_dequote_escaped(input: &str) -> IResult<&str, Cow<str>> {
    (
        sq_dequote_section,
        sq_dequote_trail,
        many0(sq_dequote_trail),
    )
        .map(|(start, trail, mut trails): (_, _, Vec<_>)| {
            trails.insert(0, trail);
            trails.insert(0, [start, ""]);
            let value = trails.into_iter().flatten().join("");
            Cow::Owned(value)
        })
        .parse_next(input)
}

fn sq_dequote_no_escaped(input: &str) -> IResult<&str, Cow<str>> {
    sq_dequote_section.map(Cow::Borrowed).parse_next(input)
}

fn sq_dequote_section(input: &str) -> IResult<&str, &str> {
    terminated(preceded('\'', take_while0(|c| c != '\'')), '\'')(input)
}

fn sq_dequote_trail(input: &str) -> IResult<&str, [&str; 2]> {
    (escaped, sq_dequote_section)
        .map(|(e, s)| [e, s])
        .parse_next(input)
}

fn escaped(input: &str) -> IResult<&str, &str> {
    preceded('\\', one_of("'!").recognize())(input)
}

#[cfg(test)]
mod test_sq_dequote_step {
    use super::*;

    #[test]
    fn word() {
        let fixture = "'name'";
        let expected = Cow::Borrowed("name");
        let (_, actual) = sq_dequote_step(fixture).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn space() {
        let fixture = "'a b'";
        let expected = Cow::Borrowed("a b");
        let (_, actual) = sq_dequote_step(fixture).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn sq_escaped() {
        let fixture = "'a'\\''b'";
        let expected: Cow<str> = Cow::Owned("a'b".into());
        let (_, actual) = sq_dequote_step(fixture).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn exclamation_escaped() {
        let fixture = "'a'\\!'b'";
        let expected: Cow<str> = Cow::Owned("a!b".into());
        let (_, actual) = sq_dequote_step(fixture).unwrap();
        assert_eq!(actual, expected);
    }
}

use std::borrow::Cow;

/// Read `GIT_CONFIG_PARAMETERS`
///
/// These are the `-c` parameters, passed from the `git` process to the subcommand.
///
/// See [parse_parameter] for how to parse the `-c` parameter.
pub struct ConfigParameters {
    values: String,
}

impl ConfigParameters {
    pub fn new() -> Self {
        let values = std::env::var("GIT_CONFIG_PARAMETERS").unwrap_or_else(|_| Default::default());
        Self { values }
    }

    pub fn iter(&self) -> ConfigParametersIter<'_> {
        self.into_iter()
    }
}

impl<'s> IntoIterator for &'s ConfigParameters {
    type Item = (Cow<'s, str>, Option<Cow<'s, str>>);
    type IntoIter = ConfigParametersIter<'s>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter::new(&self.values)
    }
}

/// Parse `GIT_CONFIG_PARAMETERS`
///
/// These are the `-c` parameters, passed from the `git` process to the subcommand.
///
/// See [parse_parameter] for how to parse the `-c` parameter.
pub struct ConfigParametersIter<'s> {
    values: &'s str,
}

impl<'s> ConfigParametersIter<'s> {
    pub fn new(values: &'s str) -> Self {
        Self { values }
    }

    pub fn iter(&self) -> impl Iterator<Item = (Cow<str>, Cow<str>)> + '_ {
        None.into_iter()
    }
}

impl<'s> Iterator for ConfigParametersIter<'s> {
    type Item = (Cow<'s, str>, Option<Cow<'s, str>>);

    fn next(&mut self) -> Option<Self::Item> {
        // See git's config.c's `parse_config_env_list`
        let (values, key) = crate::quote::sq_dequote_step(self.values.trim_start()).ok()?;
        self.values = values;

        if let Some(values) = self.values.strip_prefix('=') {
            // new-style 'key'='value'
            self.values = values;

            if self.values.is_empty() {
                Some((key, None))
            } else if let Some(values) = self.values.strip_prefix(' ') {
                self.values = values;
                Some((key, None))
            } else {
                let (values, value) = crate::quote::sq_dequote_step(self.values).ok()?;
                self.values = values;
                Some((key, Some(value)))
            }
        } else {
            // old-style 'key=value'
            if self.values.is_empty() {
                Some(parse_parameter_cow(key))
            } else if let Some(values) = self.values.strip_prefix(' ') {
                self.values = values;
                Some(parse_parameter_cow(key))
            } else {
                self.values = "";
                None
            }
        }
    }
}

#[cfg(test)]
mod test_env {
    use super::*;

    #[test]
    fn empty() {
        let fixture = "";
        let config = ConfigParametersIter::new(fixture);
        let actual: Vec<_> = config.collect();
        assert_eq!(actual, vec![]);
    }

    #[test]
    fn test_old() {
        let fixture = "'delta.plus-style=green'";
        let config = ConfigParametersIter::new(fixture);
        let actual: Vec<_> = config.collect();
        assert_eq!(
            actual,
            vec![(
                Cow::Borrowed("delta.plus-style"),
                Some(Cow::Borrowed("green"))
            )]
        );
    }

    #[test]
    fn test_old_bool() {
        let fixture = "'delta.plus-style'";
        let config = ConfigParametersIter::new(fixture);
        let actual: Vec<_> = config.collect();
        assert_eq!(actual, vec![(Cow::Borrowed("delta.plus-style"), None)]);
    }

    #[test]
    fn test_old_multiple() {
        let fixture = "'delta.plus-style=green' 'delta.plus-style' 'delta.plus-style=green'";
        let config = ConfigParametersIter::new(fixture);
        let actual: Vec<_> = config.collect();
        assert_eq!(
            actual,
            vec![
                (
                    Cow::Borrowed("delta.plus-style"),
                    Some(Cow::Borrowed("green"))
                ),
                (Cow::Borrowed("delta.plus-style"), None),
                (
                    Cow::Borrowed("delta.plus-style"),
                    Some(Cow::Borrowed("green"))
                ),
            ]
        );
    }

    #[test]
    fn test_new() {
        let fixture = "'delta.plus-style'='green'";
        let config = ConfigParametersIter::new(fixture);
        let actual: Vec<_> = config.collect();
        assert_eq!(
            actual,
            vec![(
                Cow::Borrowed("delta.plus-style"),
                Some(Cow::Borrowed("green"))
            )]
        );
    }

    #[test]
    fn test_new_bool() {
        let fixture = "'delta.plus-style'=";
        let config = ConfigParametersIter::new(fixture);
        let actual: Vec<_> = config.collect();
        assert_eq!(actual, vec![(Cow::Borrowed("delta.plus-style"), None)]);
    }

    #[test]
    fn test_new_multiple() {
        let fixture = "'delta.plus-style'='green' 'delta.plus-style'= 'delta.plus-style'='green'";
        let config = ConfigParametersIter::new(fixture);
        let actual: Vec<_> = config.collect();
        assert_eq!(
            actual,
            vec![
                (
                    Cow::Borrowed("delta.plus-style"),
                    Some(Cow::Borrowed("green"))
                ),
                (Cow::Borrowed("delta.plus-style"), None),
                (
                    Cow::Borrowed("delta.plus-style"),
                    Some(Cow::Borrowed("green"))
                ),
            ]
        );
    }
}

/// Parse a command line argument into a key/value pair
///
/// When the `value` is `None`, it is implied to be a `true` boolean entry
pub fn parse_parameter(arg: &str) -> (&str, Option<&str>) {
    // When we see:
    //
    //   section.subsection=with=equals.key=value
    //
    // we cannot tell if it means:
    //
    //   [section "subsection=with=equals"]
    //   key = value
    //
    // or:
    //
    //   [section]
    //   subsection = with=equals.key=value
    //
    // We parse left-to-right for the first "=", meaning we'll prefer to
    // keep the value intact over the subsection. This is historical, but
    // also sensible since values are more likely to contain odd or
    // untrusted input than a section name.
    //
    // A missing equals is explicitly allowed (as a bool-only entry).
    //
    // See git's config.c's `git_config_push_parameter`
    arg.split_once('=')
        .map(|(k, v)| (k, Some(v)))
        .unwrap_or((arg, None))
}

fn parse_parameter_cow(arg: Cow<str>) -> (Cow<str>, Option<Cow<str>>) {
    match arg {
        Cow::Borrowed(arg) => {
            let (key, value) = parse_parameter(arg);
            (Cow::Borrowed(key), value.map(Cow::Borrowed))
        }
        Cow::Owned(arg) => {
            let (key, value) = parse_parameter(arg.as_str());
            (
                Cow::Owned(key.to_owned()),
                value.map(|v| Cow::Owned(v.to_owned())),
            )
        }
    }
}

#[cfg(test)]
mod test_parse_parameter {
    use super::*;

    #[test]
    fn basic() {
        let fixture = "key=value";
        let expected = ("key", Some("value"));
        let actual = parse_parameter(fixture);
        assert_eq!(actual, expected);
    }

    #[test]
    fn implied_bool() {
        let fixture = "key";
        let expected = ("key", None);
        let actual = parse_parameter(fixture);
        assert_eq!(actual, expected);
    }

    #[test]
    fn multiple_eq() {
        let fixture = "section.subsection=with=equals.key=value";
        let expected = ("section.subsection", Some("with=equals.key=value"));
        let actual = parse_parameter(fixture);
        assert_eq!(actual, expected);
    }
}

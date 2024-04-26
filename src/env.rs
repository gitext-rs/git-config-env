use std::borrow::Cow;

/// Read user-defined configuration
///
/// If `GIT_CONFIG_COUNT` is set to a positive number, all environment pairs `GIT_CONFIG_KEY_<n>`
/// and `GIT_CONFIG_VALUE_<n>` up to that number will be read. The config pairs are zero-indexed.
/// Any missing key or value is will be ignored. An empty `GIT_CONFIG_COUNT` is treated the same
/// as `GIT_CONFIG_COUNT=0`, namely no pairs are processed.
///
/// These environment variables should override values in configuration files, but should be
/// overridden by any explicit options passed via `git -c`.
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub struct ConfigEnv<E: Env> {
    e: E,
}

impl ConfigEnv<StdEnv> {
    pub fn new() -> Self {
        Self { e: StdEnv }
    }
}

impl ConfigEnv<NoEnv> {
    pub fn empty() -> Self {
        Self { e: NoEnv }
    }
}

impl<E: Env> ConfigEnv<E> {
    pub fn with_env(e: E) -> Self {
        Self { e }
    }

    pub fn iter(&self) -> ConfigEnvIter<'_, E> {
        self.into_iter()
    }
}

impl<'e, E: Env> IntoIterator for &'e ConfigEnv<E> {
    type Item = (Cow<'e, str>, Cow<'e, str>);
    type IntoIter = ConfigEnvIter<'e, E>;

    fn into_iter(self) -> Self::IntoIter {
        let i = 0;
        let max = self
            .e
            .var("GIT_CONFIG_COUNT")
            .ok()
            .and_then(|m| m.parse().ok())
            .unwrap_or(0);
        Self::IntoIter { e: &self.e, max, i }
    }
}

impl<K, V> FromIterator<(K, V)> for ConfigEnv<std::collections::HashMap<String, String>>
where
    K: Into<String>,
    V: Into<String>,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let e = iter
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
        Self { e }
    }
}

/// Iterate over user-defined configuration
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ConfigEnvIter<'e, E: Env> {
    e: &'e E,
    max: usize,
    i: usize,
}

impl<'e, E: Env> Iterator for ConfigEnvIter<'e, E> {
    type Item = (Cow<'e, str>, Cow<'e, str>);

    fn next(&mut self) -> Option<Self::Item> {
        // See git's config.c's `git_config_from_parameters`
        while self.i < self.max {
            let key_key = format!("GIT_CONFIG_KEY_{}", self.i);
            let value_key = format!("GIT_CONFIG_VALUE_{}", self.i);
            self.i += 1;
            if let (Ok(key), Ok(value)) = (self.e.var(&key_key), self.e.var(&value_key)) {
                return Some((key, value));
            }
        }
        None
    }
}

/// Abstract over `std::env` for [`ConfigEnv`]
pub trait Env {
    fn var(&self, key: &str) -> Result<Cow<'_, str>, std::env::VarError>;
}

/// Use `std::env::var` for [`ConfigEnv`]
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub struct StdEnv;

impl Env for StdEnv {
    fn var(&self, key: &str) -> Result<Cow<'_, str>, std::env::VarError> {
        std::env::var(key).map(Cow::Owned)
    }
}

/// No-op env for [`ConfigEnv`]
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub struct NoEnv;

impl Env for NoEnv {
    fn var(&self, _key: &str) -> Result<Cow<'_, str>, std::env::VarError> {
        Err(std::env::VarError::NotPresent)
    }
}

impl Env for std::collections::HashMap<String, String> {
    fn var(&self, key: &str) -> Result<Cow<'_, str>, std::env::VarError> {
        self.get(key)
            .map(|v| v.as_str())
            .map(Cow::Borrowed)
            .ok_or(std::env::VarError::NotPresent)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn implicitly_empty() {
        let c = ConfigEnv::empty();
        assert_eq!(c.iter().collect::<Vec<_>>(), vec![]);
    }

    #[test]
    fn explicitly_empty() {
        let c: ConfigEnv<_> = vec![("GIT_CONFIG_COUNT", "0")].into_iter().collect();
        assert_eq!(c.iter().collect::<Vec<_>>(), vec![]);
    }

    #[test]
    fn bad_count() {
        let c: ConfigEnv<_> = vec![("GIT_CONFIG_COUNT", "")].into_iter().collect();
        assert_eq!(c.iter().collect::<Vec<_>>(), vec![]);

        let c: ConfigEnv<_> = vec![("GIT_CONFIG_COUNT", "-1")].into_iter().collect();
        assert_eq!(c.iter().collect::<Vec<_>>(), vec![]);

        let c: ConfigEnv<_> = vec![("GIT_CONFIG_COUNT", "County McCountFace")]
            .into_iter()
            .collect();
        assert_eq!(c.iter().collect::<Vec<_>>(), vec![]);
    }

    #[test]
    fn single() {
        let c: ConfigEnv<_> = vec![
            ("GIT_CONFIG_COUNT", "1"),
            ("GIT_CONFIG_KEY_0", "key"),
            ("GIT_CONFIG_VALUE_0", "value"),
        ]
        .into_iter()
        .collect();
        assert_eq!(
            c.iter().collect::<Vec<_>>(),
            vec![(Cow::Borrowed("key"), Cow::Borrowed("value"))]
        );
    }

    #[test]
    fn multiple() {
        let c: ConfigEnv<_> = vec![
            ("GIT_CONFIG_COUNT", "3"),
            ("GIT_CONFIG_KEY_0", "key"),
            ("GIT_CONFIG_VALUE_0", "value"),
            ("GIT_CONFIG_KEY_1", "one_key"),
            ("GIT_CONFIG_VALUE_1", "one_value"),
            ("GIT_CONFIG_KEY_2", "two_key"),
            ("GIT_CONFIG_VALUE_2", "two_value"),
        ]
        .into_iter()
        .collect();
        assert_eq!(
            c.iter().collect::<Vec<_>>(),
            vec![
                (Cow::Borrowed("key"), Cow::Borrowed("value")),
                (Cow::Borrowed("one_key"), Cow::Borrowed("one_value")),
                (Cow::Borrowed("two_key"), Cow::Borrowed("two_value")),
            ]
        );
    }
}

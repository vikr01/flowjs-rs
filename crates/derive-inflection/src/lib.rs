//! Case inflection for derive macros.
//!
//! Provides the standard set of `rename_all` case transformations used by
//! serde, ts-rs, flowjs-rs, and similar derive crates.
//!
//! ```
//! use derive_inflection::Inflection;
//!
//! assert_eq!(Inflection::Camel.apply("first_name"), "firstName");
//! assert_eq!(Inflection::Snake.apply("firstName"), "first_name");
//! assert_eq!(Inflection::Kebab.apply("firstName"), "first-name");
//! assert_eq!(Inflection::ScreamingSnake.apply("firstName"), "FIRST_NAME");
//! ```

/// Field/variant name inflection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Inflection {
    /// `"lowercase"` — all lowercase, no separators.
    Lower,
    /// `"UPPERCASE"` — all uppercase, no separators.
    Upper,
    /// `"camelCase"`
    Camel,
    /// `"snake_case"`
    Snake,
    /// `"PascalCase"`
    Pascal,
    /// `"SCREAMING_SNAKE_CASE"`
    ScreamingSnake,
    /// `"kebab-case"`
    Kebab,
    /// `"SCREAMING-KEBAB-CASE"`
    ScreamingKebab,
}

impl Inflection {
    /// Parse from the standard `rename_all` string values.
    ///
    /// Returns `None` for unrecognized values.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "lowercase" => Some(Self::Lower),
            "UPPERCASE" => Some(Self::Upper),
            "camelCase" => Some(Self::Camel),
            "snake_case" => Some(Self::Snake),
            "PascalCase" => Some(Self::Pascal),
            "SCREAMING_SNAKE_CASE" => Some(Self::ScreamingSnake),
            "kebab-case" => Some(Self::Kebab),
            "SCREAMING-KEBAB-CASE" => Some(Self::ScreamingKebab),
            _ => None,
        }
    }

    /// All accepted `rename_all` values.
    pub const VALID_VALUES: &[&str] = &[
        "lowercase",
        "UPPERCASE",
        "camelCase",
        "snake_case",
        "PascalCase",
        "SCREAMING_SNAKE_CASE",
        "kebab-case",
        "SCREAMING-KEBAB-CASE",
    ];

    /// Apply the inflection to a string.
    pub fn apply(&self, s: &str) -> String {
        match self {
            Self::Lower => s.to_lowercase(),
            Self::Upper => s.to_uppercase(),
            Self::Snake => to_snake_case(s),
            Self::ScreamingSnake => to_snake_case(s).to_uppercase(),
            Self::Camel => to_camel_case(s),
            Self::Pascal => to_pascal_case(s),
            Self::Kebab => to_snake_case(s).replace('_', "-"),
            Self::ScreamingKebab => to_snake_case(s).to_uppercase().replace('_', "-"),
        }
    }
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap());
        } else {
            result.push(ch);
        }
    }
    result
}

fn to_camel_case(s: &str) -> String {
    let pascal = to_pascal_case(s);
    let mut chars = pascal.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_lowercase().to_string() + chars.as_str(),
    }
}

fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().to_string() + chars.as_str(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_all_values() {
        for val in Inflection::VALID_VALUES {
            assert!(Inflection::parse(val).is_some(), "should parse: {val}");
        }
        assert!(Inflection::parse("invalid").is_none());
    }

    #[test]
    fn camel_case() {
        assert_eq!(Inflection::Camel.apply("first_name"), "firstName");
        assert_eq!(Inflection::Camel.apply("FirstName"), "firstName");
    }

    #[test]
    fn snake_case() {
        assert_eq!(Inflection::Snake.apply("firstName"), "first_name");
        assert_eq!(Inflection::Snake.apply("FirstName"), "first_name");
    }

    #[test]
    fn pascal_case() {
        assert_eq!(Inflection::Pascal.apply("first_name"), "FirstName");
    }

    #[test]
    fn kebab_case() {
        assert_eq!(Inflection::Kebab.apply("first_name"), "first-name");
        assert_eq!(Inflection::Kebab.apply("FirstName"), "first-name");
    }

    #[test]
    fn screaming_snake() {
        assert_eq!(Inflection::ScreamingSnake.apply("firstName"), "FIRST_NAME");
    }

    #[test]
    fn screaming_kebab() {
        assert_eq!(Inflection::ScreamingKebab.apply("firstName"), "FIRST-NAME");
    }

    #[test]
    fn lower_upper() {
        assert_eq!(Inflection::Lower.apply("FooBar"), "foobar");
        assert_eq!(Inflection::Upper.apply("FooBar"), "FOOBAR");
    }
}

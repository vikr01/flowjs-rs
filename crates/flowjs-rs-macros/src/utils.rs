//! Shared utilities for the derive macro.

/// Escape a string for use as a Flow string literal value (inside single quotes).
pub fn escape_string_literal(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "\\'")
}

/// Quote a Flow property name if it contains non-identifier characters.
/// Flow requires non-identifier keys to be quoted: `'my-field'` instead of `my-field`.
/// Single quotes inside the name are escaped.
pub fn quote_property_name(name: &str) -> String {
    let is_identifier = !name.is_empty()
        && name.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '$')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$');

    if is_identifier {
        name.to_owned()
    } else {
        let escaped = name.replace('\\', "\\\\").replace('\'', "\\'");
        format!("'{escaped}'")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifier_names_unquoted() {
        // Arrange and Act and Assert
        assert_eq!(quote_property_name("name"), "name", "simple identifier");
        assert_eq!(quote_property_name("firstName"), "firstName", "camelCase");
        assert_eq!(quote_property_name("_private"), "_private", "underscore prefix");
        assert_eq!(quote_property_name("$ref"), "$ref", "dollar prefix");
    }

    #[test]
    fn non_identifier_names_quoted() {
        // Arrange and Act and Assert
        assert_eq!(quote_property_name("my-field"), "'my-field'", "kebab-case");
        assert_eq!(quote_property_name("MY-FIELD"), "'MY-FIELD'", "screaming kebab");
        assert_eq!(quote_property_name("has space"), "'has space'", "space in name");
        assert_eq!(quote_property_name("123start"), "'123start'", "starts with digit");
    }

    #[test]
    fn quotes_escaped_in_names() {
        // Arrange and Act and Assert
        assert_eq!(quote_property_name("can't"), "'can\\'t'", "single quote escaped");
    }
}

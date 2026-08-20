//! File format for the project configuration.
//!
//! The config is a JSON object mapping each project's script file name to a
//! [`ProjectConfig`]. See [`Config`].

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A single entry in the project configuration file.
///
/// Maps one project script (the map key it lives under in [`Config`]) to its
/// display metadata. `parent` and `icon` are optional and default to `None`
/// when absent.
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ProjectConfig {
    /// Display name of the project.
    pub name: String,
    /// Optional parent/group the project belongs to.
    #[serde(default)]
    pub parent: Option<String>,
    /// Optional icon (e.g. an emoji) shown next to the project.
    #[serde(default)]
    pub icon: Option<String>,
}

/// The full configuration: each project script file name mapped to its settings.
///
/// Keys are script file names (e.g. `"a.sh"`).
pub type Config = BTreeMap<String, ProjectConfig>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_empty_object() {
        let config: Config = serde_json::from_str("{}").unwrap();
        assert!(config.is_empty());
    }

    #[test]
    fn deserialize_defaults_missing_parent_and_icon() {
        let config: Config = serde_json::from_str(r#"{ "a.sh": { "name": "A" } }"#).unwrap();
        assert_eq!(config.len(), 1);
        let a = &config["a.sh"];
        assert_eq!(a.name, "A");
        assert!(a.parent.is_none());
        assert!(a.icon.is_none());
    }

    #[test]
    fn deserialize_preserves_parent_and_icon() {
        let config: Config = serde_json::from_str(
            r#"{
                "a.sh": {
                    "name": "A",
                    "parent": "backend",
                    "icon": "🐘"
                }
            }"#,
        )
        .unwrap();
        let a = &config["a.sh"];
        assert_eq!(a.parent.as_deref(), Some("backend"));
        assert_eq!(a.icon.as_deref(), Some("\u{1F418}"));
    }

    #[test]
    fn serialize_round_trip() {
        let mut config = Config::new();
        config.insert(
            "a.sh".to_string(),
            ProjectConfig {
                name: "A".into(),
                parent: Some("backend".into()),
                icon: Some("\u{1F418}".into()),
            },
        );
        config.insert(
            "b.sh".to_string(),
            ProjectConfig {
                name: "B".into(),
                parent: None,
                icon: None,
            },
        );

        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: Config = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, config);
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let config: Config =
            serde_json::from_str(r#"{ "a.sh": { "name": "A", "color": "red" } }"#).unwrap();
        assert_eq!(config["a.sh"].name, "A");
        assert!(config["a.sh"].parent.is_none());
    }
}

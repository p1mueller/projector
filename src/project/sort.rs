use super::Project;

/// Field the overview list can be ordered by.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    #[default]
    Name,
    Parent,
    Script,
}

impl SortMode {
    /// Order the given projects by this mode's key (case-insensitive, with
    /// missing `parent` values last, ties broken by name).
    pub fn apply(self, projects: &mut [Project]) {
        match self {
            Self::Name => projects.sort_by(|a, b| a.name.cmp(&b.name)),
            Self::Parent => {
                projects.sort_by(|a, b| match (a.parent.as_deref(), b.parent.as_deref()) {
                    (Some(pa), Some(pb)) => pa
                        .to_lowercase()
                        .cmp(&pb.to_lowercase())
                        .then_with(|| a.name.cmp(&b.name)),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => a.name.cmp(&b.name),
                })
            }
            Self::Script => projects.sort_by(|a, b| a.script_name.cmp(&b.script_name)),
        }
    }

    /// The next mode in the cycle.
    pub fn next(self) -> Self {
        match self {
            Self::Name => Self::Parent,
            Self::Parent => Self::Script,
            Self::Script => Self::Name,
        }
    }

    /// A short label for the footer/status line.
    pub fn label(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Parent => "parent",
            Self::Script => "script",
        }
    }
}

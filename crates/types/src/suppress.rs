//! Inline suppression comment types and issue kind definitions.

/// Issue kind for suppression matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueKind {
    /// An unused file.
    UnusedFile,
    /// An unused export.
    UnusedExport,
    /// An unused type export.
    UnusedType,
    /// An unused dependency.
    UnusedDependency,
    /// An unused dev dependency.
    UnusedDevDependency,
    /// An unused enum member.
    UnusedEnumMember,
    /// An unused class member.
    UnusedClassMember,
    /// An unresolved import.
    UnresolvedImport,
    /// An unlisted dependency.
    UnlistedDependency,
    /// A duplicate export name across modules.
    DuplicateExport,
    /// Code duplication.
    CodeDuplication,
    /// A circular dependency chain.
    CircularDependency,
}

impl IssueKind {
    /// Parse an issue kind from the string tokens used in CLI output and suppression comments.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "unused-file" => Some(Self::UnusedFile),
            "unused-export" => Some(Self::UnusedExport),
            "unused-type" => Some(Self::UnusedType),
            "unused-dependency" => Some(Self::UnusedDependency),
            "unused-dev-dependency" => Some(Self::UnusedDevDependency),
            "unused-enum-member" => Some(Self::UnusedEnumMember),
            "unused-class-member" => Some(Self::UnusedClassMember),
            "unresolved-import" => Some(Self::UnresolvedImport),
            "unlisted-dependency" => Some(Self::UnlistedDependency),
            "duplicate-export" => Some(Self::DuplicateExport),
            "code-duplication" => Some(Self::CodeDuplication),
            "circular-dependency" => Some(Self::CircularDependency),
            _ => None,
        }
    }

    /// Convert to a u8 discriminant for compact cache storage.
    pub const fn to_discriminant(self) -> u8 {
        match self {
            Self::UnusedFile => 1,
            Self::UnusedExport => 2,
            Self::UnusedType => 3,
            Self::UnusedDependency => 4,
            Self::UnusedDevDependency => 5,
            Self::UnusedEnumMember => 6,
            Self::UnusedClassMember => 7,
            Self::UnresolvedImport => 8,
            Self::UnlistedDependency => 9,
            Self::DuplicateExport => 10,
            Self::CodeDuplication => 11,
            Self::CircularDependency => 12,
        }
    }

    /// Reconstruct from a cache discriminant.
    pub const fn from_discriminant(d: u8) -> Option<Self> {
        match d {
            1 => Some(Self::UnusedFile),
            2 => Some(Self::UnusedExport),
            3 => Some(Self::UnusedType),
            4 => Some(Self::UnusedDependency),
            5 => Some(Self::UnusedDevDependency),
            6 => Some(Self::UnusedEnumMember),
            7 => Some(Self::UnusedClassMember),
            8 => Some(Self::UnresolvedImport),
            9 => Some(Self::UnlistedDependency),
            10 => Some(Self::DuplicateExport),
            11 => Some(Self::CodeDuplication),
            12 => Some(Self::CircularDependency),
            _ => None,
        }
    }
}

/// A suppression directive parsed from a source comment.
#[derive(Debug, Clone)]
pub struct Suppression {
    /// 1-based line this suppression applies to. 0 = file-wide suppression.
    pub line: u32,
    /// None = suppress all issue kinds on this line.
    pub kind: Option<IssueKind>,
}

// Size assertions to prevent memory regressions.
// `Suppression` is stored in a Vec per file; `IssueKind` appears in every suppression.
const _: () = assert!(std::mem::size_of::<Suppression>() == 8);
const _: () = assert!(std::mem::size_of::<IssueKind>() == 1);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_kind_from_str_all_variants() {
        assert_eq!(IssueKind::parse("unused-file"), Some(IssueKind::UnusedFile));
        assert_eq!(
            IssueKind::parse("unused-export"),
            Some(IssueKind::UnusedExport)
        );
        assert_eq!(IssueKind::parse("unused-type"), Some(IssueKind::UnusedType));
        assert_eq!(
            IssueKind::parse("unused-dependency"),
            Some(IssueKind::UnusedDependency)
        );
        assert_eq!(
            IssueKind::parse("unused-dev-dependency"),
            Some(IssueKind::UnusedDevDependency)
        );
        assert_eq!(
            IssueKind::parse("unused-enum-member"),
            Some(IssueKind::UnusedEnumMember)
        );
        assert_eq!(
            IssueKind::parse("unused-class-member"),
            Some(IssueKind::UnusedClassMember)
        );
        assert_eq!(
            IssueKind::parse("unresolved-import"),
            Some(IssueKind::UnresolvedImport)
        );
        assert_eq!(
            IssueKind::parse("unlisted-dependency"),
            Some(IssueKind::UnlistedDependency)
        );
        assert_eq!(
            IssueKind::parse("duplicate-export"),
            Some(IssueKind::DuplicateExport)
        );
        assert_eq!(
            IssueKind::parse("circular-dependency"),
            Some(IssueKind::CircularDependency)
        );
    }

    #[test]
    fn issue_kind_from_str_unknown() {
        assert_eq!(IssueKind::parse("foo"), None);
        assert_eq!(IssueKind::parse(""), None);
    }

    #[test]
    fn discriminant_roundtrip() {
        for kind in [
            IssueKind::UnusedFile,
            IssueKind::UnusedExport,
            IssueKind::UnusedType,
            IssueKind::UnusedDependency,
            IssueKind::UnusedDevDependency,
            IssueKind::UnusedEnumMember,
            IssueKind::UnusedClassMember,
            IssueKind::UnresolvedImport,
            IssueKind::UnlistedDependency,
            IssueKind::DuplicateExport,
            IssueKind::CodeDuplication,
            IssueKind::CircularDependency,
        ] {
            assert_eq!(
                IssueKind::from_discriminant(kind.to_discriminant()),
                Some(kind)
            );
        }
        assert_eq!(IssueKind::from_discriminant(0), None);
        assert_eq!(IssueKind::from_discriminant(13), None);
    }
}

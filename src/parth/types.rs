use std::collections::HashMap;
use std::cmp::Ordering;
use std::fmt;

/// A dependency specification: name + version constraint (as string, preserved from source)
#[derive(Debug, Clone, PartialEq)]
pub struct PkgDep {
    pub name: String,
    pub version_req: String,
}

/// A locked package entry
#[derive(Debug, Clone, PartialEq)]
pub struct LockEntry {
    pub version: String,
    pub checksum: String,
    pub dependencies: Vec<PkgDep>,
}

/// The lock file, keyed by package name
pub type LockFile = HashMap<String, LockEntry>;

/// Registry index: package_name → version → checksum
pub type RegistryIndex = HashMap<String, HashMap<String, String>>;

/// A parsed semantic version (major.minor.patch)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    /// Parse a version string. Accepts "X.Y.Z" (3 parts) or "X.Y" (2 parts, patch=0).
    /// Rejects empty strings, non-numeric parts, and 4+ part versions.
    pub fn parse(s: &str) -> Option<Self> {
        if s.is_empty() {
            return None;
        }
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() < 2 || parts.len() > 3 {
            return None;
        }
        let major = parts[0].parse::<u32>().ok()?;
        let minor = parts[1].parse::<u32>().ok()?;
        let patch = parts.get(2).map(|s| s.parse::<u32>().ok()).unwrap_or(Some(0))?;
        Some(Version { major, minor, patch })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.major.cmp(&other.major) {
            Ordering::Equal => {}
            ord => return ord,
        }
        match self.minor.cmp(&other.minor) {
            Ordering::Equal => {}
            ord => return ord,
        }
        self.patch.cmp(&other.patch)
    }
}

/// A parsed version constraint
#[derive(Debug, Clone, PartialEq)]
pub enum VersionConstraint {
    Any,
    Exact(Version),
    Caret(Version),
    Gte(Version),
    Gt(Version),
    Lte(Version),
    Lt(Version),
}

impl fmt::Display for VersionConstraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VersionConstraint::Any => write!(f, "*"),
            VersionConstraint::Exact(v) => write!(f, "{}", v),
            VersionConstraint::Caret(v) => write!(f, "^{}", v),
            VersionConstraint::Gte(v) => write!(f, ">={}", v),
            VersionConstraint::Gt(v) => write!(f, ">{}", v),
            VersionConstraint::Lte(v) => write!(f, "<={}", v),
            VersionConstraint::Lt(v) => write!(f, "<{}", v),
        }
    }
}

impl VersionConstraint {
    /// Parse a constraint string. Returns None for unparseable constraints.
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if s.is_empty() || s == "*" {
            return Some(VersionConstraint::Any);
        }
        if let Some(base) = s.strip_prefix(">=") {
            let base = base.trim();
            return Some(VersionConstraint::Gte(Version::parse(base)?));
        }
        if let Some(base) = s.strip_prefix("<=") {
            let base = base.trim();
            return Some(VersionConstraint::Lte(Version::parse(base)?));
        }
        if let Some(base) = s.strip_prefix('^') {
            let base = base.trim();
            return Some(VersionConstraint::Caret(Version::parse(base)?));
        }
        if let Some(base) = s.strip_prefix('>') {
            let base = base.trim();
            return Some(VersionConstraint::Gt(Version::parse(base)?));
        }
        if let Some(base) = s.strip_prefix('<') {
            let base = base.trim();
            return Some(VersionConstraint::Lt(Version::parse(base)?));
        }
        // Exact version
        Some(VersionConstraint::Exact(Version::parse(s)?))
    }

    /// Check whether a version satisfies this constraint.
    pub fn matches(&self, version: &Version) -> bool {
        match self {
            VersionConstraint::Any => true,
            VersionConstraint::Exact(v) => version == v,
            VersionConstraint::Caret(base) => {
                version.major == base.major && version >= base
            }
            VersionConstraint::Gte(base) => version >= base,
            VersionConstraint::Gt(base) => version > base,
            VersionConstraint::Lte(base) => version <= base,
            VersionConstraint::Lt(base) => version < base,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Version parsing ──────────────────────────────────────────────

    #[test]
    fn test_version_parse_valid() {
        let v = Version::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }

    #[test]
    fn test_version_parse_zero() {
        let v = Version::parse("0.0.0").unwrap();
        assert_eq!(v, Version { major: 0, minor: 0, patch: 0 });
    }

    #[test]
    fn test_version_parse_large() {
        let v = Version::parse("999999.888888.777777").unwrap();
        assert_eq!(v.major, 999999);
        assert_eq!(v.minor, 888888);
        assert_eq!(v.patch, 777777);
    }

    #[test]
    fn test_version_parse_two_part() {
        let v = Version::parse("1.2").unwrap();
        assert_eq!(v, Version { major: 1, minor: 2, patch: 0 });
        let v = Version::parse("2.0").unwrap();
        assert_eq!(v, Version { major: 2, minor: 0, patch: 0 });
    }

    #[test]
    fn test_version_parse_invalid() {
        assert!(Version::parse("").is_none());
        assert!(Version::parse("1.2.3.4").is_none());
        assert!(Version::parse("abc").is_none());
        assert!(Version::parse("1.2.x").is_none());
        assert!(Version::parse("1.2.3-alpha").is_none());
        assert!(Version::parse("1").is_none());
    }

    // ── Version ordering ─────────────────────────────────────────────

    #[test]
    fn test_version_ord_equal() {
        assert_eq!(Version::parse("1.2.3").unwrap(), Version::parse("1.2.3").unwrap());
    }

    #[test]
    fn test_version_ord_major() {
        assert!(Version::parse("2.0.0").unwrap() > Version::parse("1.9.9").unwrap());
        assert!(Version::parse("1.0.0").unwrap() < Version::parse("2.0.0").unwrap());
    }

    #[test]
    fn test_version_ord_minor() {
        assert!(Version::parse("1.10.0").unwrap() > Version::parse("1.9.0").unwrap());
        assert!(Version::parse("1.9.0").unwrap() < Version::parse("1.10.0").unwrap());
    }

    #[test]
    fn test_version_ord_patch() {
        assert!(Version::parse("1.0.10").unwrap() > Version::parse("1.0.9").unwrap());
        assert!(Version::parse("1.0.9").unwrap() < Version::parse("1.0.10").unwrap());
    }

    #[test]
    fn test_version_sort_newest_first() {
        let mut versions = vec![
            Version::parse("1.9.0").unwrap(),
            Version::parse("1.10.0").unwrap(),
            Version::parse("1.0.0").unwrap(),
            Version::parse("2.0.0").unwrap(),
        ];
        versions.sort_by(|a, b| b.cmp(a));
        assert_eq!(versions[0].to_string(), "2.0.0");
        assert_eq!(versions[1].to_string(), "1.10.0");
        assert_eq!(versions[2].to_string(), "1.9.0");
        assert_eq!(versions[3].to_string(), "1.0.0");
    }

    // ── Version Display ──────────────────────────────────────────────

    #[test]
    fn test_version_display() {
        assert_eq!(Version::parse("1.2.3").unwrap().to_string(), "1.2.3");
        assert_eq!(Version::parse("0.0.0").unwrap().to_string(), "0.0.0");
        assert_eq!(Version::parse("999.888.777").unwrap().to_string(), "999.888.777");
    }

    // ── Constraint parsing ───────────────────────────────────────────

    #[test]
    fn test_constraint_parse_any() {
        let c = VersionConstraint::parse("*").unwrap();
        assert!(matches!(c, VersionConstraint::Any));
        let c = VersionConstraint::parse("").unwrap();
        assert!(matches!(c, VersionConstraint::Any));
    }

    #[test]
    fn test_constraint_parse_exact() {
        let c = VersionConstraint::parse("1.2.3").unwrap();
        assert!(matches!(c, VersionConstraint::Exact(_)));
        if let VersionConstraint::Exact(v) = c {
            assert_eq!(v, Version::parse("1.2.3").unwrap());
        }
    }

    #[test]
    fn test_constraint_parse_caret() {
        let c = VersionConstraint::parse("^1.2.3").unwrap();
        assert!(matches!(c, VersionConstraint::Caret(_)));
    }

    #[test]
    fn test_constraint_parse_gte() {
        let c = VersionConstraint::parse(">=1.2.3").unwrap();
        assert!(matches!(c, VersionConstraint::Gte(_)));
    }

    #[test]
    fn test_constraint_parse_gt() {
        let c = VersionConstraint::parse(">1.2.3").unwrap();
        assert!(matches!(c, VersionConstraint::Gt(_)));
    }

    #[test]
    fn test_constraint_parse_lte() {
        let c = VersionConstraint::parse("<=1.2.3").unwrap();
        assert!(matches!(c, VersionConstraint::Lte(_)));
    }

    #[test]
    fn test_constraint_parse_lt() {
        let c = VersionConstraint::parse("<1.2.3").unwrap();
        assert!(matches!(c, VersionConstraint::Lt(_)));
    }

    #[test]
    fn test_constraint_parse_invalid() {
        assert!(VersionConstraint::parse(">=abc").is_none());
        assert!(VersionConstraint::parse("^x.y.z").is_none());
    }

    // ── Constraint matching ──────────────────────────────────────────

    #[test]
    fn test_any_matches_all() {
        let c = VersionConstraint::Any;
        assert!(c.matches(&Version::parse("0.0.0").unwrap()));
        assert!(c.matches(&Version::parse("999.999.999").unwrap()));
    }

    #[test]
    fn test_exact_matching() {
        let c = VersionConstraint::Exact(Version::parse("1.2.3").unwrap());
        assert!(c.matches(&Version::parse("1.2.3").unwrap()));
        assert!(!c.matches(&Version::parse("1.2.4").unwrap()));
        assert!(!c.matches(&Version::parse("2.0.0").unwrap()));
    }

    #[test]
    fn test_caret_matching() {
        let c = VersionConstraint::Caret(Version::parse("1.2.3").unwrap());
        assert!(c.matches(&Version::parse("1.2.3").unwrap()));
        assert!(c.matches(&Version::parse("1.9.9").unwrap()));
        assert!(c.matches(&Version::parse("1.20.0").unwrap()));
        assert!(!c.matches(&Version::parse("2.0.0").unwrap()));
        assert!(!c.matches(&Version::parse("0.9.9").unwrap()));
        assert!(c.matches(&Version::parse("1.2.4").unwrap()));
        assert!(!c.matches(&Version::parse("1.2.2").unwrap()));
    }

    #[test]
    fn test_caret_lower() {
        let c = VersionConstraint::Caret(Version::parse("0.1.0").unwrap());
        assert!(c.matches(&Version::parse("0.1.0").unwrap()));
        assert!(c.matches(&Version::parse("0.2.0").unwrap()));
        assert!(!c.matches(&Version::parse("0.0.9").unwrap()));
        assert!(!c.matches(&Version::parse("1.0.0").unwrap()));
    }

    #[test]
    fn test_gte_matching() {
        let c = VersionConstraint::Gte(Version::parse("1.5.0").unwrap());
        assert!(c.matches(&Version::parse("1.5.0").unwrap()));
        assert!(c.matches(&Version::parse("2.0.0").unwrap()));
        assert!(c.matches(&Version::parse("1.10.0").unwrap()));
        assert!(!c.matches(&Version::parse("1.4.9").unwrap()));
        assert!(!c.matches(&Version::parse("0.9.0").unwrap()));
    }

    #[test]
    fn test_gt_matching() {
        let c = VersionConstraint::Gt(Version::parse("1.5.0").unwrap());
        assert!(!c.matches(&Version::parse("1.5.0").unwrap()));
        assert!(c.matches(&Version::parse("1.5.1").unwrap()));
        assert!(c.matches(&Version::parse("2.0.0").unwrap()));
    }

    #[test]
    fn test_lte_matching() {
        let c = VersionConstraint::Lte(Version::parse("1.5.0").unwrap());
        assert!(c.matches(&Version::parse("1.5.0").unwrap()));
        assert!(c.matches(&Version::parse("1.4.9").unwrap()));
        assert!(!c.matches(&Version::parse("1.5.1").unwrap()));
        assert!(!c.matches(&Version::parse("2.0.0").unwrap()));
    }

    #[test]
    fn test_lt_matching() {
        let c = VersionConstraint::Lt(Version::parse("1.5.0").unwrap());
        assert!(!c.matches(&Version::parse("1.5.0").unwrap()));
        assert!(c.matches(&Version::parse("1.4.9").unwrap()));
        assert!(!c.matches(&Version::parse("2.0.0").unwrap()));
    }

    // ── Multi-digit semver (the bug we fixed) ────────────────────────

    #[test]
    fn test_multi_digit_semver_gte() {
        let c = VersionConstraint::Gte(Version::parse("1.9.0").unwrap());
        assert!(c.matches(&Version::parse("1.10.0").unwrap()));
        assert!(c.matches(&Version::parse("1.9.0").unwrap()));
        assert!(!c.matches(&Version::parse("1.8.9").unwrap()));
    }

    #[test]
    fn test_multi_digit_semver_lt() {
        let c = VersionConstraint::Lt(Version::parse("1.10.0").unwrap());
        assert!(c.matches(&Version::parse("1.9.0").unwrap()));
        assert!(!c.matches(&Version::parse("1.10.0").unwrap()));
        assert!(!c.matches(&Version::parse("1.11.0").unwrap()));
    }

    #[test]
    fn test_multi_digit_semver_caret() {
        let c = VersionConstraint::Caret(Version::parse("1.9.0").unwrap());
        assert!(c.matches(&Version::parse("1.10.0").unwrap()));
        assert!(c.matches(&Version::parse("1.9.0").unwrap()));
        assert!(!c.matches(&Version::parse("2.0.0").unwrap()));
    }

    #[test]
    fn test_multi_digit_semver_sort() {
        let v1 = Version::parse("1.9.0").unwrap();
        let v2 = Version::parse("1.10.0").unwrap();
        assert!(v2 > v1);
        assert!(v1 < v2);
    }

    // ── Constraint Display ───────────────────────────────────────────

    #[test]
    fn test_constraint_display_any() {
        assert_eq!(VersionConstraint::Any.to_string(), "*");
    }

    #[test]
    fn test_constraint_display_exact() {
        let c = VersionConstraint::Exact(Version::parse("1.2.3").unwrap());
        assert_eq!(c.to_string(), "1.2.3");
    }

    #[test]
    fn test_constraint_display_caret() {
        let c = VersionConstraint::Caret(Version::parse("1.2.3").unwrap());
        assert_eq!(c.to_string(), "^1.2.3");
    }

    #[test]
    fn test_constraint_display_gte() {
        let c = VersionConstraint::Gte(Version::parse("1.2.3").unwrap());
        assert_eq!(c.to_string(), ">=1.2.3");
    }
}

use std::collections::HashMap;
use std::cmp::Ordering;
use std::fmt;

/// A dependency specification
#[derive(Debug, Clone, PartialEq)]
pub struct PkgDep {
    pub name: String,
    pub version_req: String,
}

/// Lock file entry (v2 — stores resolved transitive deps)
#[derive(Debug, Clone, PartialEq)]
pub struct LockEntry {
    pub version: String,
    pub checksum: String,
    pub dependencies: Vec<PkgDep>,
    pub registry: String,
}

pub type LockFile = HashMap<String, LockEntry>;

/// Registry index: package_name → version → (checksum, metadata_json)
pub type RegistryIndex = HashMap<String, HashMap<String, String>>;

/// Registry package metadata stored alongside the index
#[derive(Debug, Clone, PartialEq)]
pub struct RegistryMetadata {
    pub description: String,
    pub author: String,
    pub homepage: String,
    pub license: String,
    pub yanked: bool,
}

impl RegistryMetadata {
    pub fn new(_name: &str) -> Self {
        RegistryMetadata {
            description: String::new(),
            author: String::new(),
            homepage: String::new(),
            license: String::new(),
            yanked: false,
        }
    }
}

/// Workspace definition
#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceMember {
    pub path: String,
}

/// Feature definition
#[derive(Debug, Clone, PartialEq)]
pub struct Feature {
    pub name: String,
    pub deps: Vec<String>,
}

/// Build profile
#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    pub name: String,
    pub opt_level: u8,
    pub debug: bool,
    pub lto: bool,
}

impl Default for Profile {
    fn default() -> Self {
        Profile { name: "dev".into(), opt_level: 0, debug: true, lto: false }
    }
}

impl Profile {
    pub fn release() -> Self {
        Profile { name: "release".into(), opt_level: 3, debug: false, lto: true }
    }
}

/// Ed25519-based package signature
#[derive(Debug, Clone, PartialEq)]
pub struct PackageSignature {
    pub signer: String,
    pub hash: String,
    /// Hex-encoded Ed25519 signature
    pub signature_hex: String,
    /// Hex-encoded Ed25519 public key of the signer
    pub public_key_hex: String,
    pub timestamp: u64,
}

/// Security advisory
#[cfg_attr(feature = "remote-registry", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct Advisory {
    pub id: String,
    pub package: String,
    pub severity: String,
    pub versions_affected: String,
    pub description: String,
}

/// Search result
#[cfg_attr(feature = "remote-registry", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    pub name: String,
    pub latest_version: String,
    pub description: String,
}

/// A parsed semantic version
#[derive(Debug, Clone, Eq)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub pre: Vec<String>,
    pub build: Vec<String>,
}

impl Version {
    pub fn parse(s: &str) -> Option<Self> {
        if s.is_empty() { return None; }
        let (without_build, build) = if let Some(plus) = s.find('+') {
            let b: Vec<String> = s[plus + 1..].split('.').map(|p| p.to_string()).collect();
            (&s[..plus], b)
        } else { (s, Vec::new()) };
        let (core, pre) = if let Some(hyphen) = without_build.find('-') {
            let p: Vec<String> = without_build[hyphen + 1..].split('.').map(|s| s.to_string()).collect();
            (&without_build[..hyphen], p)
        } else { (without_build, Vec::new()) };
        let parts: Vec<&str> = core.split('.').collect();
        if parts.len() < 2 || parts.len() > 3 { return None; }
        let major = parts[0].parse::<u32>().ok()?;
        let minor = parts[1].parse::<u32>().ok()?;
        let patch = parts.get(2).map(|s| s.parse::<u32>().ok()).unwrap_or(Some(0))?;
        Some(Version { major, minor, patch, pre, build })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if !self.pre.is_empty() { write!(f, "-{}", self.pre.join("."))?; }
        if !self.build.is_empty() { write!(f, "+{}", self.build.join("."))?; }
        Ok(())
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool { self.cmp(other) == Ordering::Equal }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.major.cmp(&other.major) { Ordering::Equal => {} ord => return ord }
        match self.minor.cmp(&other.minor) { Ordering::Equal => {} ord => return ord }
        match self.patch.cmp(&other.patch) { Ordering::Equal => {} ord => return ord }
        match (self.pre.is_empty(), other.pre.is_empty()) {
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (false, false) => {
                for (a, b) in self.pre.iter().zip(other.pre.iter()) {
                    let cmp = compare_pre_ident(a, b);
                    if cmp != Ordering::Equal { return cmp; }
                }
                self.pre.len().cmp(&other.pre.len())
            }
            (true, true) => Ordering::Equal,
        }
    }
}

fn compare_pre_ident(a: &str, b: &str) -> Ordering {
    match (a.parse::<u32>(), b.parse::<u32>()) {
        (Ok(ai), Ok(bi)) => ai.cmp(&bi),
        (Ok(_), Err(_)) => Ordering::Less,
        (Err(_), Ok(_)) => Ordering::Greater,
        (Err(_), Err(_)) => a.cmp(b),
    }
}

/// Compound version constraint: supports AND/OR combinators
#[derive(Debug, Clone, PartialEq)]
pub enum VersionConstraint {
    Any,
    Exact(Version),
    Caret(Version),
    Tilde(Version),
    Gte(Version),
    Gt(Version),
    Lte(Version),
    Lt(Version),
    And(Box<VersionConstraint>, Box<VersionConstraint>),
    Or(Box<VersionConstraint>, Box<VersionConstraint>),
}

impl fmt::Display for VersionConstraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VersionConstraint::Any => write!(f, "*"),
            VersionConstraint::Exact(v) => write!(f, "={}", v),
            VersionConstraint::Caret(v) => write!(f, "^{}", v),
            VersionConstraint::Tilde(v) => write!(f, "~{}", v),
            VersionConstraint::Gte(v) => write!(f, ">={}", v),
            VersionConstraint::Gt(v) => write!(f, ">{}", v),
            VersionConstraint::Lte(v) => write!(f, "<={}", v),
            VersionConstraint::Lt(v) => write!(f, "<{}", v),
            VersionConstraint::And(a, b) => write!(f, "{} && {}", a, b),
            VersionConstraint::Or(a, b) => write!(f, "{} || {}", a, b),
        }
    }
}

impl VersionConstraint {
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if s.is_empty() || s == "*" { return Some(VersionConstraint::Any); }
        // Handle OR: `>=1.0.0 <2.0.0 || >=3.0.0`
        if let Some(pipe) = s.find("||") {
            let left = VersionConstraint::parse(s[..pipe].trim())?;
            let right = VersionConstraint::parse(s[pipe + 2..].trim())?;
            return Some(VersionConstraint::Or(Box::new(left), Box::new(right)));
        }
        // Handle AND / implicit AND: `>=1.0.0 <2.0.0` or `>=1.0.0 && <2.0.0`
        let sub_constraints = if s.contains("&&") {
            s.split("&&").map(|p| p.trim()).collect()
        } else {
            // Try splitting on spaces if there are multiple constraint-like segments
            let tokens: Vec<&str> = s.split_whitespace().collect();
            if tokens.len() > 1 && tokens.iter().all(|t| Self::is_constraint(t)) {
                tokens
            } else {
                vec![s]
            }
        };

        if sub_constraints.len() > 1 {
            let mut iter = sub_constraints.iter();
            let first = VersionConstraint::parse_single(iter.next()?)?;
            let mut result = first;
            for part in iter {
                let next = VersionConstraint::parse_single(part)?;
                result = VersionConstraint::And(Box::new(result), Box::new(next));
            }
            return Some(result);
        }
        Self::parse_single(s)
    }

    fn is_constraint(s: &str) -> bool {
        s.starts_with('^') || s.starts_with('~') || s.starts_with('>') 
        || s.starts_with('<') || s.starts_with('=') || s == "*"
    }

    fn parse_single(s: &str) -> Option<Self> {
        if s.is_empty() || s == "*" { return Some(VersionConstraint::Any); }
        if let Some(base) = s.strip_prefix(">=").map(|b| b.trim()) { return Some(VersionConstraint::Gte(Version::parse(base)?)); }
        if let Some(base) = s.strip_prefix("<=").map(|b| b.trim()) { return Some(VersionConstraint::Lte(Version::parse(base)?)); }
        if let Some(base) = s.strip_prefix("~").map(|b| b.trim()) { return Some(VersionConstraint::Tilde(Version::parse(base)?)); }
        if let Some(base) = s.strip_prefix('^').map(|b| b.trim()) { return Some(VersionConstraint::Caret(Version::parse(base)?)); }
        if let Some(base) = s.strip_prefix('=').map(|b| b.trim()) { return Some(VersionConstraint::Exact(Version::parse(base)?)); }
        if let Some(base) = s.strip_prefix('>').map(|b| b.trim()) { return Some(VersionConstraint::Gt(Version::parse(base)?)); }
        if let Some(base) = s.strip_prefix('<').map(|b| b.trim()) { return Some(VersionConstraint::Lt(Version::parse(base)?)); }
        Some(VersionConstraint::Exact(Version::parse(s)?))
    }

    pub fn matches(&self, version: &Version) -> bool {
        match self {
            VersionConstraint::Any => true,
            VersionConstraint::Exact(v) => version == v,
            VersionConstraint::Caret(base) => {
                if version.major != base.major { return false; }
                if version.major == 0 && version.minor != base.minor { return false; }
                version >= base
            }
            VersionConstraint::Tilde(base) => {
                version.major == base.major && version.minor == base.minor && version >= base
            }
            VersionConstraint::Gte(base) => version >= base,
            VersionConstraint::Gt(base) => version > base,
            VersionConstraint::Lte(base) => version <= base,
            VersionConstraint::Lt(base) => version < base,
            VersionConstraint::And(a, b) => a.matches(version) && b.matches(version),
            VersionConstraint::Or(a, b) => a.matches(version) || b.matches(version),
        }
    }

    /// Lower bound for this constraint (for PubGrub solver)
    pub fn lower_bound(&self) -> Option<&Version> {
        match self {
            VersionConstraint::Exact(v) | VersionConstraint::Caret(v) 
            | VersionConstraint::Tilde(v) | VersionConstraint::Gte(v) 
            | VersionConstraint::Gt(v) => Some(v),
            VersionConstraint::And(a, b) => {
                let (la, lb) = (a.lower_bound(), b.lower_bound());
                match (la, lb) {
                    (Some(va), Some(vb)) => Some(if va >= vb { va } else { vb }),
                    (Some(v), None) | (None, Some(v)) => Some(v),
                    (None, None) => None,
                }
            }
            _ => None,
        }
    }

    /// Human-readable conflict explanation
    pub fn description(&self) -> String {
        match self {
            VersionConstraint::Any => "any version".into(),
            VersionConstraint::Exact(v) => format!("exactly {}", v),
            VersionConstraint::Caret(v) => format!("^{}", v),
            VersionConstraint::Tilde(v) => format!("~{}", v),
            VersionConstraint::Gte(v) => format!(">= {}", v),
            VersionConstraint::Gt(v) => format!("> {}", v),
            VersionConstraint::Lte(v) => format!("<= {}", v),
            VersionConstraint::Lt(v) => format!("< {}", v),
            VersionConstraint::And(a, b) => format!("({} and {})", a.description(), b.description()),
            VersionConstraint::Or(a, b) => format!("({} or {})", a.description(), b.description()),
        }
    }
}

/// A decision in the resolution trail (for PubGrub/incremental solver)
#[derive(Debug, Clone, PartialEq)]
pub struct Decision {
    pub package: String,
    pub version: Version,
    pub dependencies: Vec<PkgDep>,
    pub level: usize,
}

/// Conflict info for backtracking
#[derive(Debug, Clone, PartialEq)]
pub struct Conflict {
    pub package: String,
    pub cause: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parse() {
        let v = Version::parse("1.2.3").unwrap();
        assert_eq!(v, Version { major: 1, minor: 2, patch: 3, pre: vec![], build: vec![] });
        assert!(Version::parse("1.2.3-alpha.1+build.5").is_some());
        assert_eq!(Version::parse("1.2.3-alpha.1+build.5").unwrap().pre, vec!["alpha".to_string(), "1".to_string()]);
        assert!(Version::parse("").is_none());
    }

    #[test]
    fn test_compound_ranges() {
        let c = VersionConstraint::parse(">=1.0.0 <2.0.0").unwrap();
        assert!(c.matches(&Version::parse("1.5.0").unwrap()));
        assert!(!c.matches(&Version::parse("2.0.0").unwrap()));
        assert!(!c.matches(&Version::parse("0.9.0").unwrap()));

        let c = VersionConstraint::parse("^1.0.0 || ^2.0.0").unwrap();
        assert!(c.matches(&Version::parse("1.5.0").unwrap()));
        assert!(c.matches(&Version::parse("2.5.0").unwrap()));
        assert!(!c.matches(&Version::parse("3.0.0").unwrap()));
    }

    #[test]
    fn test_pre_release() {
        assert!(Version::parse("1.0.0-alpha").unwrap() < Version::parse("1.0.0").unwrap());
        assert!(Version::parse("1.0.0-beta").unwrap() > Version::parse("1.0.0-alpha").unwrap());
    }

    #[test]
    fn test_constraint_description() {
        let c = VersionConstraint::parse(">=1.0.0 <2.0.0").unwrap();
        let desc = c.description();
        assert!(desc.contains(">= 1.0.0") || desc.contains("1.0.0"));
    }
}

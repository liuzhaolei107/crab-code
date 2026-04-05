//! MCP protocol version negotiation.
//!
//! Provides [`ProtocolVersion`] for semantic versioning of the MCP protocol,
//! version range constraints, and negotiation logic to find the highest
//! mutually supported version.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

/// A semantic protocol version: `MAJOR.MINOR.PATCH`.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl ProtocolVersion {
    /// The current MCP protocol version implemented by this crate.
    pub const CURRENT: Self = Self::new(2024, 11, 5);

    #[must_use]
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Whether two versions share the same major version (API-compatible).
    #[must_use]
    pub fn is_major_compatible(&self, other: &Self) -> bool {
        self.major == other.major
    }
}

impl fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Ord for ProtocolVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
    }
}

impl PartialOrd for ProtocolVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Error when parsing a version string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseProtocolVersionError(pub String);

impl fmt::Display for ParseProtocolVersionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid protocol version: {}", self.0)
    }
}

impl std::error::Error for ParseProtocolVersionError {}

impl FromStr for ProtocolVersion {
    type Err = ParseProtocolVersionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(ParseProtocolVersionError(s.to_string()));
        }
        let major = parts[0]
            .parse()
            .map_err(|_| ParseProtocolVersionError(s.to_string()))?;
        let minor = parts[1]
            .parse()
            .map_err(|_| ParseProtocolVersionError(s.to_string()))?;
        let patch = parts[2]
            .parse()
            .map_err(|_| ParseProtocolVersionError(s.to_string()))?;
        Ok(Self::new(major, minor, patch))
    }
}

/// A version range constraint: `[min, max]` inclusive.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct VersionRange {
    pub min: ProtocolVersion,
    pub max: ProtocolVersion,
}

impl VersionRange {
    #[must_use]
    pub fn new(min: ProtocolVersion, max: ProtocolVersion) -> Self {
        Self { min, max }
    }

    /// A range covering exactly one version.
    #[must_use]
    pub fn exact(version: ProtocolVersion) -> Self {
        Self {
            min: version,
            max: version,
        }
    }

    /// Check if a version falls within this range.
    #[must_use]
    pub fn contains(&self, version: &ProtocolVersion) -> bool {
        *version >= self.min && *version <= self.max
    }

    /// Check if two ranges overlap.
    #[must_use]
    pub fn overlaps(&self, other: &Self) -> bool {
        self.min <= other.max && other.min <= self.max
    }

    /// Compute the intersection of two ranges, if any.
    #[must_use]
    pub fn intersect(&self, other: &Self) -> Option<Self> {
        let min = std::cmp::max(self.min, other.min);
        let max = std::cmp::min(self.max, other.max);
        if min <= max {
            Some(Self { min, max })
        } else {
            None
        }
    }
}

impl fmt::Display for VersionRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}, {}]", self.min, self.max)
    }
}

/// Result of version negotiation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NegotiationResult {
    /// Successfully negotiated a version.
    Agreed(ProtocolVersion),
    /// No mutually supported version found.
    NoCommonVersion,
}

impl NegotiationResult {
    /// Get the agreed version, if any.
    #[must_use]
    pub fn version(&self) -> Option<ProtocolVersion> {
        match self {
            Self::Agreed(v) => Some(*v),
            Self::NoCommonVersion => None,
        }
    }

    #[must_use]
    pub fn is_agreed(&self) -> bool {
        matches!(self, Self::Agreed(_))
    }
}

/// Negotiate the highest mutually supported version from two lists of
/// supported versions.
///
/// Both lists are searched for the highest version present in both.
#[must_use]
pub fn negotiate_version(
    client_versions: &[ProtocolVersion],
    server_versions: &[ProtocolVersion],
) -> NegotiationResult {
    let mut common: Vec<ProtocolVersion> = client_versions
        .iter()
        .filter(|v| server_versions.contains(v))
        .copied()
        .collect();

    if common.is_empty() {
        return NegotiationResult::NoCommonVersion;
    }

    common.sort();
    NegotiationResult::Agreed(*common.last().unwrap())
}

/// Negotiate the highest version within overlapping ranges.
#[must_use]
pub fn negotiate_version_range(
    client_range: &VersionRange,
    server_range: &VersionRange,
) -> NegotiationResult {
    client_range.intersect(server_range).map_or(
        NegotiationResult::NoCommonVersion,
        |intersection| NegotiationResult::Agreed(intersection.max),
    )
}

/// Check if a specific feature is available at a given protocol version.
#[derive(Debug, Clone)]
pub struct CompatibilityCheck {
    feature_name: String,
    min_version: ProtocolVersion,
}

impl CompatibilityCheck {
    #[must_use]
    pub fn new(feature_name: impl Into<String>, min_version: ProtocolVersion) -> Self {
        Self {
            feature_name: feature_name.into(),
            min_version,
        }
    }

    /// Name of the feature.
    #[must_use]
    pub fn feature_name(&self) -> &str {
        &self.feature_name
    }

    /// Minimum version required.
    #[must_use]
    pub fn min_version(&self) -> ProtocolVersion {
        self.min_version
    }

    /// Check if the feature is available at the given version.
    #[must_use]
    pub fn is_available(&self, version: &ProtocolVersion) -> bool {
        *version >= self.min_version
    }
}

/// Registry of feature compatibility checks.
#[derive(Debug, Default)]
pub struct CompatibilityRegistry {
    checks: Vec<CompatibilityCheck>,
}

impl CompatibilityRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a feature with its minimum required version.
    pub fn register(&mut self, check: CompatibilityCheck) {
        self.checks.push(check);
    }

    /// Get all features available at a given version.
    #[must_use]
    pub fn available_features(&self, version: &ProtocolVersion) -> Vec<&str> {
        self.checks
            .iter()
            .filter(|c| c.is_available(version))
            .map(CompatibilityCheck::feature_name)
            .collect()
    }

    /// Check if a specific feature is available.
    #[must_use]
    pub fn is_feature_available(&self, feature: &str, version: &ProtocolVersion) -> bool {
        self.checks
            .iter()
            .any(|c| c.feature_name == feature && c.is_available(version))
    }

    /// Get all registered feature names.
    #[must_use]
    pub fn all_features(&self) -> Vec<&str> {
        self.checks.iter().map(CompatibilityCheck::feature_name).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_parse_and_display() {
        let v: ProtocolVersion = "2024.11.5".parse().unwrap();
        assert_eq!(v, ProtocolVersion::new(2024, 11, 5));
        assert_eq!(v.to_string(), "2024.11.5");
    }

    #[test]
    fn version_parse_error() {
        assert!("1.2".parse::<ProtocolVersion>().is_err());
        assert!("abc".parse::<ProtocolVersion>().is_err());
    }

    #[test]
    fn version_ordering() {
        let v1 = ProtocolVersion::new(2024, 11, 0);
        let v2 = ProtocolVersion::new(2024, 11, 5);
        let v3 = ProtocolVersion::new(2025, 0, 0);
        assert!(v1 < v2);
        assert!(v2 < v3);
    }

    #[test]
    fn version_major_compatible() {
        let a = ProtocolVersion::new(2024, 11, 0);
        let b = ProtocolVersion::new(2024, 12, 0);
        let c = ProtocolVersion::new(2025, 0, 0);
        assert!(a.is_major_compatible(&b));
        assert!(!a.is_major_compatible(&c));
    }

    #[test]
    fn version_serde_roundtrip() {
        let v = ProtocolVersion::new(2024, 11, 5);
        let json = serde_json::to_string(&v).unwrap();
        let back: ProtocolVersion = serde_json::from_str(&json).unwrap();
        assert_eq!(v, back);
    }

    #[test]
    fn version_range_contains() {
        let r = VersionRange::new(
            ProtocolVersion::new(1, 0, 0),
            ProtocolVersion::new(2, 0, 0),
        );
        assert!(r.contains(&ProtocolVersion::new(1, 5, 0)));
        assert!(r.contains(&ProtocolVersion::new(1, 0, 0)));
        assert!(r.contains(&ProtocolVersion::new(2, 0, 0)));
        assert!(!r.contains(&ProtocolVersion::new(0, 9, 0)));
        assert!(!r.contains(&ProtocolVersion::new(2, 0, 1)));
    }

    #[test]
    fn version_range_exact() {
        let r = VersionRange::exact(ProtocolVersion::new(1, 0, 0));
        assert!(r.contains(&ProtocolVersion::new(1, 0, 0)));
        assert!(!r.contains(&ProtocolVersion::new(1, 0, 1)));
    }

    #[test]
    fn version_range_overlaps() {
        let r1 = VersionRange::new(
            ProtocolVersion::new(1, 0, 0),
            ProtocolVersion::new(2, 0, 0),
        );
        let r2 = VersionRange::new(
            ProtocolVersion::new(1, 5, 0),
            ProtocolVersion::new(3, 0, 0),
        );
        let r3 = VersionRange::new(
            ProtocolVersion::new(3, 0, 0),
            ProtocolVersion::new(4, 0, 0),
        );
        assert!(r1.overlaps(&r2));
        assert!(!r1.overlaps(&r3));
    }

    #[test]
    fn version_range_intersect() {
        let r1 = VersionRange::new(
            ProtocolVersion::new(1, 0, 0),
            ProtocolVersion::new(2, 0, 0),
        );
        let r2 = VersionRange::new(
            ProtocolVersion::new(1, 5, 0),
            ProtocolVersion::new(3, 0, 0),
        );
        let i = r1.intersect(&r2).unwrap();
        assert_eq!(i.min, ProtocolVersion::new(1, 5, 0));
        assert_eq!(i.max, ProtocolVersion::new(2, 0, 0));
    }

    #[test]
    fn version_range_no_intersect() {
        let r1 = VersionRange::new(
            ProtocolVersion::new(1, 0, 0),
            ProtocolVersion::new(1, 9, 0),
        );
        let r2 = VersionRange::new(
            ProtocolVersion::new(2, 0, 0),
            ProtocolVersion::new(3, 0, 0),
        );
        assert!(r1.intersect(&r2).is_none());
    }

    #[test]
    fn version_range_display() {
        let r = VersionRange::new(
            ProtocolVersion::new(1, 0, 0),
            ProtocolVersion::new(2, 0, 0),
        );
        assert_eq!(r.to_string(), "[1.0.0, 2.0.0]");
    }

    #[test]
    fn negotiate_version_success() {
        let client = vec![
            ProtocolVersion::new(1, 0, 0),
            ProtocolVersion::new(2, 0, 0),
        ];
        let server = vec![
            ProtocolVersion::new(2, 0, 0),
            ProtocolVersion::new(3, 0, 0),
        ];
        let result = negotiate_version(&client, &server);
        assert_eq!(result, NegotiationResult::Agreed(ProtocolVersion::new(2, 0, 0)));
        assert!(result.is_agreed());
    }

    #[test]
    fn negotiate_version_picks_highest() {
        let client = vec![
            ProtocolVersion::new(1, 0, 0),
            ProtocolVersion::new(2, 0, 0),
            ProtocolVersion::new(3, 0, 0),
        ];
        let server = vec![
            ProtocolVersion::new(1, 0, 0),
            ProtocolVersion::new(2, 0, 0),
            ProtocolVersion::new(3, 0, 0),
        ];
        assert_eq!(
            negotiate_version(&client, &server).version(),
            Some(ProtocolVersion::new(3, 0, 0))
        );
    }

    #[test]
    fn negotiate_version_no_common() {
        let client = vec![ProtocolVersion::new(1, 0, 0)];
        let server = vec![ProtocolVersion::new(2, 0, 0)];
        let result = negotiate_version(&client, &server);
        assert_eq!(result, NegotiationResult::NoCommonVersion);
        assert!(!result.is_agreed());
        assert!(result.version().is_none());
    }

    #[test]
    fn negotiate_version_range_success() {
        let client = VersionRange::new(
            ProtocolVersion::new(1, 0, 0),
            ProtocolVersion::new(2, 5, 0),
        );
        let server = VersionRange::new(
            ProtocolVersion::new(2, 0, 0),
            ProtocolVersion::new(3, 0, 0),
        );
        let result = negotiate_version_range(&client, &server);
        // Picks max of intersection: 2.5.0
        assert_eq!(result.version(), Some(ProtocolVersion::new(2, 5, 0)));
    }

    #[test]
    fn negotiate_version_range_no_overlap() {
        let client = VersionRange::new(
            ProtocolVersion::new(1, 0, 0),
            ProtocolVersion::new(1, 9, 0),
        );
        let server = VersionRange::new(
            ProtocolVersion::new(2, 0, 0),
            ProtocolVersion::new(3, 0, 0),
        );
        assert_eq!(
            negotiate_version_range(&client, &server),
            NegotiationResult::NoCommonVersion
        );
    }

    #[test]
    fn compatibility_check() {
        let check = CompatibilityCheck::new("sampling", ProtocolVersion::new(2024, 11, 0));
        assert!(check.is_available(&ProtocolVersion::new(2024, 11, 5)));
        assert!(check.is_available(&ProtocolVersion::new(2024, 11, 0)));
        assert!(!check.is_available(&ProtocolVersion::new(2024, 10, 0)));
        assert_eq!(check.feature_name(), "sampling");
    }

    #[test]
    fn compatibility_registry() {
        let mut reg = CompatibilityRegistry::new();
        reg.register(CompatibilityCheck::new("tools", ProtocolVersion::new(1, 0, 0)));
        reg.register(CompatibilityCheck::new("sampling", ProtocolVersion::new(2, 0, 0)));
        reg.register(CompatibilityCheck::new("roots", ProtocolVersion::new(2, 1, 0)));

        let v1 = ProtocolVersion::new(1, 5, 0);
        assert_eq!(reg.available_features(&v1), vec!["tools"]);

        let v2 = ProtocolVersion::new(2, 1, 0);
        let features = reg.available_features(&v2);
        assert_eq!(features.len(), 3);

        assert!(reg.is_feature_available("tools", &v2));
        assert!(!reg.is_feature_available("roots", &v1));
        assert_eq!(reg.all_features().len(), 3);
    }

    #[test]
    fn current_version() {
        assert_eq!(ProtocolVersion::CURRENT, ProtocolVersion::new(2024, 11, 5));
    }
}

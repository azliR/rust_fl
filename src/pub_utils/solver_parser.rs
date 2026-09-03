use std::collections::HashSet;

use regex::Regex;

/// Represents an SDK package pinned to a fixed version by an SDK component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinnedSdkPackage {
    pub package: String,
    pub version: String,
    pub owner: String,
    pub sdk: String,
}

/// Represents a package requiring a specific Dart SDK range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkRequirementConstraint {
    pub package: String,
    pub version: String,
    pub sdk_range: String,
}

/// Represents a mutual incompatibility between two packages found by the solver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncompatibilityPair {
    pub package_a: String,
    pub version_range_a: String,
    pub package_b: String,
    pub version_range_b: String,
}

/// Represents the final deadlock causing PubGrub version solving to fail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootDeadlock {
    pub project: String,
    pub package_a: String,
    pub version_range_a: String,
    pub package_b: String,
    pub version_range_b: String,
}

/// Aggregated report from parsing PubGrub solver error output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PubDiagnosisReport {
    pub pinned_packages: Vec<PinnedSdkPackage>,
    pub current_dart_sdk_version: Option<String>,
    pub sdk_constraints: Vec<SdkRequirementConstraint>,
    pub incompatibilities: Vec<IncompatibilityPair>,
    pub root_deadlock: Option<RootDeadlock>,
    pub raw_bottlenecks: Vec<String>,
}

impl PubDiagnosisReport {
    pub fn has_issues(&self) -> bool {
        self.root_deadlock.is_some()
            || !self.incompatibilities.is_empty()
            || !self.pinned_packages.is_empty()
            || !self.sdk_constraints.is_empty()
    }

    pub fn generate_recommendations(&self) -> Vec<String> {
        let mut recommendations = Vec::new();

        if let Some(ref deadlock) = self.root_deadlock {
            recommendations.push(format!(
                "Direct conflict between '{}' ({}) and '{}' ({}).",
                deadlock.package_a,
                deadlock.version_range_a,
                deadlock.package_b,
                deadlock.version_range_b
            ));
            recommendations.push(format!(
                "Try loosening or pinning '{}' to an earlier compatible version in pubspec.yaml.",
                deadlock.package_a
            ));
            recommendations.push(format!(
                "Alternatively, adjust '{}' constraints if '{}' is strictly required.",
                deadlock.package_b, deadlock.package_a
            ));
        }

        if !self.pinned_packages.is_empty() {
            let pinned_summary = self
                .pinned_packages
                .iter()
                .map(|p| format!("{} ({})", p.package, p.version))
                .collect::<Vec<_>>()
                .join(", ");
            recommendations.push(format!(
                "The Flutter SDK is pinning: {pinned_summary}. Any transitive dependency requiring newer versions of these will fail to resolve until the Flutter SDK is updated."
            ));
        }

        if !self.sdk_constraints.is_empty() {
            let constraint_summary = self
                .sdk_constraints
                .iter()
                .map(|c| format!("{} {} (requires SDK {})", c.package, c.version, c.sdk_range))
                .collect::<Vec<_>>()
                .join(", ");
            let current_suffix = match &self.current_dart_sdk_version {
                Some(v) => format!(" (current is {v})"),
                None => String::new(),
            };
            recommendations.push(format!(
                "Packages requiring newer Dart SDK: {constraint_summary}{current_suffix}. Upgrade Flutter / Dart SDK or hold back these packages."
            ));
        }

        recommendations
    }
}

/// Parses PubGrub failure output and extracts structured diagnostic information.
pub fn parse_pub_solver_error(output: &str) -> PubDiagnosisReport {
    let mut pinned_packages = Vec::new();
    let mut sdk_constraints = Vec::new();
    let mut incompatibilities = Vec::new();
    let mut raw_bottlenecks = Vec::new();
    let mut root_deadlock = None;
    let mut current_dart_sdk_version = None;

    let mut seen_pinned = HashSet::new();
    let mut seen_sdk_constraints = HashSet::new();
    let mut seen_incompatibilities = HashSet::new();

    let whitespace_regex = Regex::new(r"\s+").expect("Valid regex");
    let collapsed = whitespace_regex.replace_all(output, " ");

    let pinned_regex = Regex::new(
        r"(?i)Note:\s+([a-zA-Z0-9_]+)\s+is\s+pinned\s+to\s+version\s+([^\s]+)\s+by\s+([^\s]+)\s+from\s+the\s+([^\s]+)\s+SDK"
    ).expect("Valid regex");

    let current_sdk_regex =
        Regex::new(r"(?i)The current Dart SDK version is\s+([^\s\.]+.[^\s\.]+.[^\s\.]+)")
            .expect("Valid regex");

    let sdk_constraint_regex = Regex::new(
        r"(?i)([a-zA-Z0-9_]+)\s+([^\s]+)\s+(?:which\s+)?requires\s+SDK\s+version\s+([^\n\r]+?)(?:,\s|\sand\s|\.\s|\.$)"
    ).expect("Valid regex");

    let root_deadlock_regex = Regex::new(
        r"(?i)So, because\s+([a-zA-Z0-9_]+)\s+depends\s+on\s+both\s+([a-zA-Z0-9_]+)\s+(.+?)\s+and\s+([a-zA-Z0-9_]+)\s+(.+?),\s+version\s+solving\s+failed"
    ).expect("Valid regex");

    let incompatibility_regex = Regex::new(
        r"(?i)([a-zA-Z0-9_]+)\s+([^\s]+)\s+is\s+incompatible\s+with\s+([a-zA-Z0-9_]+)\s+([^\s,]+?)\.?(?:\s|$)"
    ).expect("Valid regex");

    let bottleneck_regex =
        Regex::new(r"(?i)one\s+of\s+([^\.]+?)\s+must\s+be\s+false").expect("Valid regex");

    for captures in pinned_regex.captures_iter(output) {
        let pkg = captures[1].to_string();
        let ver = captures[2].to_string();
        let owner = captures[3].to_string();
        let sdk = captures[4].to_string();

        if seen_pinned.insert(pkg.clone()) {
            pinned_packages.push(PinnedSdkPackage {
                package: pkg,
                version: ver,
                owner,
                sdk,
            });
        }
    }

    if let Some(captures) = current_sdk_regex.captures(output) {
        current_dart_sdk_version = Some(captures[1].trim().to_string());
    }

    for captures in sdk_constraint_regex.captures_iter(&collapsed) {
        let pkg = captures[1].to_string();
        let ver = captures[2].to_string();
        let sdk_range = captures[3].trim().to_string();
        let key = format!("{pkg}:{ver}:{sdk_range}");

        if seen_sdk_constraints.insert(key) {
            sdk_constraints.push(SdkRequirementConstraint {
                package: pkg,
                version: ver,
                sdk_range,
            });
        }
    }

    if let Some(captures) = root_deadlock_regex.captures(&collapsed) {
        root_deadlock = Some(RootDeadlock {
            project: captures[1].to_string(),
            package_a: captures[2].to_string(),
            version_range_a: captures[3].to_string(),
            package_b: captures[4].to_string(),
            version_range_b: captures[5].to_string(),
        });
    }

    for captures in incompatibility_regex.captures_iter(&collapsed) {
        let pkg_a = captures[1].to_string();
        let ver_a = captures[2].to_string();
        let pkg_b = captures[3].to_string();
        let ver_b = captures[4].to_string();
        let key = format!("{pkg_a}:{pkg_b}");

        if seen_incompatibilities.insert(key) {
            incompatibilities.push(IncompatibilityPair {
                package_a: pkg_a,
                version_range_a: ver_a,
                package_b: pkg_b,
                version_range_b: ver_b,
            });
        }
    }

    for captures in bottleneck_regex.captures_iter(&collapsed) {
        raw_bottlenecks.push(captures[1].trim().to_string());
    }

    PubDiagnosisReport {
        pinned_packages,
        current_dart_sdk_version,
        sdk_constraints,
        incompatibilities,
        root_deadlock,
        raw_bottlenecks,
    }
}

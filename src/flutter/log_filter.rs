use regex::Regex;

/// Default noise patterns commonly emitted by Android OEM graphics drivers and system services.
pub const DEFAULT_IGNORED_PATTERNS: &[&str] =
    &["Empty SMPTE 2094-40 data", r"gralloc\d*.*Empty SMPTE"];

/// Filter for suppressing unwanted log messages based on regexes or substrings.
#[derive(Debug, Clone)]
pub struct LogFilter {
    regexes: Vec<Regex>,
    literals: Vec<String>,
    ansi_regex: Regex,
}

impl Default for LogFilter {
    fn default() -> Self {
        Self::new(&[], false)
    }
}

impl LogFilter {
    pub fn new(patterns: &[String], include_defaults: bool) -> Self {
        let mut regexes = Vec::new();
        let mut literals = Vec::new();

        if include_defaults {
            for pattern in DEFAULT_IGNORED_PATTERNS {
                if let Ok(re) = Regex::new(&format!("(?i){pattern}")) {
                    regexes.push(re);
                } else {
                    literals.push(pattern.to_lowercase());
                }
            }
        }

        for pattern in patterns {
            let clean = pattern.trim().trim_matches(['"', '\'']).trim();
            if clean.is_empty() {
                continue;
            }
            match Regex::new(&format!("(?i){clean}")) {
                Ok(re) => regexes.push(re),
                Err(_) => literals.push(clean.to_lowercase()),
            }
        }

        let ansi_regex = Regex::new(r"\x1B\[[0-?]*[ -/]*[@-~]").expect("Valid regex");

        Self {
            regexes,
            literals,
            ansi_regex,
        }
    }

    pub fn should_ignore(&self, text: &str) -> bool {
        if self.regexes.is_empty() && self.literals.is_empty() {
            return false;
        }

        if self.matches_candidate(text) {
            return true;
        }

        if text.contains('\x1B') {
            let stripped = self.ansi_regex.replace_all(text, "");
            if self.matches_candidate(&stripped) {
                return true;
            }
        }

        false
    }

    fn matches_candidate(&self, candidate: &str) -> bool {
        let lower = candidate.to_lowercase();
        for lit in &self.literals {
            if lower.contains(lit) {
                return true;
            }
        }

        for re in &self.regexes {
            if re.is_match(candidate) {
                return true;
            }
        }

        false
    }
}

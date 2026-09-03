use regex::Regex;

/// Filter for suppressing unwanted log messages based on regexes or substrings.
#[derive(Debug, Clone, Default)]
pub struct LogFilter {
    regexes: Vec<Regex>,
    literals: Vec<String>,
}

impl LogFilter {
    pub fn new(patterns: &[String]) -> Self {
        let mut regexes = Vec::new();
        let mut literals = Vec::new();

        for pattern in patterns {
            let trimmed = pattern.trim();
            if trimmed.is_empty() {
                continue;
            }
            match Regex::new(&format!("(?i){trimmed}")) {
                Ok(re) => regexes.push(re),
                Err(_) => literals.push(trimmed.to_lowercase()),
            }
        }

        Self { regexes, literals }
    }

    pub fn should_ignore(&self, text: &str) -> bool {
        if self.regexes.is_empty() && self.literals.is_empty() {
            return false;
        }

        let lower = text.to_lowercase();
        for lit in &self.literals {
            if lower.contains(lit) {
                return true;
            }
        }

        for re in &self.regexes {
            if re.is_match(text) {
                return true;
            }
        }

        false
    }
}

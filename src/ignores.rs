use std::path::Path;

use anyhow::Context;
use regex::Regex;
use serde::Deserialize;

#[derive(Deserialize)]
struct Ignores(Vec<String>);

pub struct IgnoreMatcher {
    patterns: Vec<Regex>,
}

impl IgnoreMatcher {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)
            .context("failed to read ignores file")?;
        let ignores: Ignores = serde_yaml::from_str(&content)
            .context("failed to parse ignores YAML")?;
        let patterns = ignores
            .0
            .into_iter()
            .map(|p| Regex::new(&p).context("invalid regex in ignores"))
            .collect::<anyhow::Result<Vec<_>>>()?;
        Ok(Self { patterns })
    }

    pub fn should_ignore(&self, path: &Path) -> bool {
        let s = path.to_string_lossy();
        self.patterns.iter().any(|re| re.is_match(&s))
    }

    pub fn empty() -> Self {
        Self {
            patterns: Vec::new(),
        }
    }
}

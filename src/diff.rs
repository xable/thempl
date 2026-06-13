use std::io::Write;

use anyhow::Context;
use colored::Colorize;
use similar::{ChangeTag, TextDiff};

pub fn show(template: &str, dest: &str, rendered: &str) -> anyhow::Result<()> {
    let dest_content = std::fs::read_to_string(dest).unwrap_or_default();
    let rendered_owned = rendered.to_string();
    let diff = TextDiff::from_lines(&dest_content, &rendered_owned);

    let mut output = Vec::new();
    write!(
        output,
        "=== {} ===\n--- {}\n+++ {} (rendered)\n",
        template, dest, dest
    )?;

    for change in diff.iter_all_changes() {
        let line = match change.tag() {
            ChangeTag::Equal => format!(" {}", change.value()),
            ChangeTag::Insert => format!("{}", format!("+{}", change.value()).green()),
            ChangeTag::Delete => format!("{}", format!("-{}", change.value()).red()),
        };
        output.extend_from_slice(line.as_bytes());
    }

    let pager = std::env::var("PAGER").unwrap_or_else(|_| "less".to_string());
    let mut child = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("{} -R 2>/dev/null || cat", pager))
        .stdin(std::process::Stdio::piped())
        .spawn()
        .context("failed to launch pager")?;

    if let Some(ref mut stdin) = child.stdin {
        stdin.write_all(&output)?;
    }
    child.wait()?;
    Ok(())
}

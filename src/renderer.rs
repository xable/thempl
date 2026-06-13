use std::path::Path;

use anyhow::Context;
use serde_json::Value;
use walkdir::WalkDir;

use crate::config::Config;
use crate::ignores::IgnoreMatcher;

pub struct Renderer {
    config: Config,
    tera: tera::Tera,
    ctx: tera::Context,
    ignores: IgnoreMatcher,
}

impl Renderer {
    pub fn new(
        config: Config,
        tera: tera::Tera,
        variables: serde_json::Map<String, Value>,
        ignores: IgnoreMatcher,
    ) -> Self {
        let ctx = tera::Context::from_serialize(&Value::Object(variables))
            .expect("failed to build Tera context from variables");
        Self {
            config,
            tera,
            ctx,
            ignores,
        }
    }

    pub fn render_pairs(&self) -> Vec<(std::path::PathBuf, std::path::PathBuf)> {
        WalkDir::new(&self.config.templates_path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| !self.ignores.should_ignore(e.path()))
            .map(|e| {
                let relative = e.path().strip_prefix(&self.config.templates_path).unwrap();
                let dest = self.config.dest_path.join(relative);
                (e.path().to_path_buf(), dest)
            })
            .collect()
    }

    pub fn render(&self, template_path: &Path) -> anyhow::Result<String> {
        let relative = template_path
            .strip_prefix(&self.config.templates_path)
            .context("template path outside template directory")?;
        let relative = relative.to_string_lossy().replace('\\', "/");
        self.tera
            .render(&relative, &self.ctx)
            .context(format!("failed to render {}", template_path.display()))
    }

    pub fn render_and_write(&self) -> anyhow::Result<()> {
        for (template, dest) in self.render_pairs() {
            let result = self.render(&template)?;
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)
                    .context(format!("failed to create directory {}", parent.display()))?;
            }
            std::fs::write(&dest, &result)
                .context(format!("failed to write {}", dest.display()))?;
            if let Ok(meta) = std::fs::metadata(&template) {
                let _ = std::fs::set_permissions(&dest, meta.permissions());
            }
            log::info!("rendered {}", dest.display());
        }
        Ok(())
    }

    pub fn dry_run(&self) -> anyhow::Result<()> {
        for (template, dest) in self.render_pairs() {
            match self.render(&template) {
                Ok(_) => log::info!("would render {}", dest.display()),
                Err(e) => log::error!("error rendering {}: {}", dest.display(), e),
            }
        }
        Ok(())
    }

    pub fn diff_all(&self) -> anyhow::Result<()> {
        for (template, dest) in self.render_pairs() {
            match self.render(&template) {
                Ok(rendered) => {
                    let template_name = template.to_string_lossy().to_string();
                    let dest_str = dest.to_string_lossy().to_string();
                    crate::diff::show(&template_name, &dest_str, &rendered)?;
                }
                Err(e) => log::error!("error rendering {}: {}", template.display(), e),
            }
        }
        Ok(())
    }
}

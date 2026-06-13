use std::path::Path;

use anyhow::Context;
use serde_json::{Map, Value};

fn load_yaml_file(path: &Path) -> anyhow::Result<Value> {
    let content = std::fs::read_to_string(path)
        .context(format!("failed to read {}", path.display()))?;
    let value: Value = serde_yaml::from_str(&content)
        .context(format!("failed to parse YAML in {}", path.display()))?;
    Ok(value)
}

fn deep_merge(base: &mut Value, overlay: Value) {
    if let Value::Object(base_map) = base {
        if let Value::Object(overlay_map) = overlay {
            for (k, v) in overlay_map {
                if let Some(existing) = base_map.get_mut(&k) {
                    deep_merge(existing, v);
                } else {
                    base_map.insert(k, v);
                }
            }
            return;
        }
    }
    *base = overlay;
}

fn merge_into_map(base: &mut Map<String, Value>, overlay: Value) {
    let mut merged = Value::Object(std::mem::take(base));
    deep_merge(&mut merged, overlay);
    if let Value::Object(m) = merged {
        *base = m;
    }
}

pub fn load(
    defaults_path: Option<&Path>,
    var_set_path: Option<&Path>,
    variable_files: &[std::path::PathBuf],
    use_env_vars: bool,
    tera: &mut tera::Tera,
) -> anyhow::Result<Map<String, Value>> {
    let mut vars = Map::new();

    // 1. env vars (lowest priority)
    if use_env_vars {
        for (k, v) in std::env::vars() {
            vars.insert(k, Value::String(v));
        }
    }

    // 2. defaults.yaml
    if let Some(path) = defaults_path {
        if path.exists() {
            let v = load_yaml_file(path)?;
            merge_into_map(&mut vars, v);
        }
    }

    // 3. variable files (in order, each overrides previous)
    for file in variable_files {
        let path = resolve_var_file(file, var_set_path)?;
        let v = load_yaml_file(&path)?;
        merge_into_map(&mut vars, v);
    }

    // 4. shallow resolve variable references (e.g. "{{ colors.background }}")
    shallow_render(&mut vars, tera)?;

    Ok(vars)
}

fn resolve_var_file(file: &Path, var_set_path: Option<&Path>) -> anyhow::Result<std::path::PathBuf> {
    if file.exists() {
        return Ok(file.to_path_buf());
    }
    if let Some(var_set_dir) = var_set_path {
        let with_ext = var_set_dir.join(file).with_extension("yaml");
        if with_ext.exists() {
            return Ok(with_ext);
        }
    }
    anyhow::bail!("variable file not found: {}", file.display())
}

fn shallow_render(vars: &mut Map<String, Value>, tera: &mut tera::Tera) -> anyhow::Result<()> {
    for _ in 0..3 {
        if !render_pass(vars, tera)? {
            break;
        }
    }
    Ok(())
}

fn render_pass(vars: &mut Map<String, Value>, tera: &mut tera::Tera) -> anyhow::Result<bool> {
    let ctx = tera::Context::from_serialize(&Value::Object(vars.clone()))
        .context("failed to build Tera context in render_pass")?;

    let mut changed = false;
    let snapshot = vars.clone();
    for (k, v) in snapshot.iter() {
        if let Value::String(s) = v {
            if s.contains("{{") {
                match tera.render_str(s, &ctx) {
                    Ok(rendered) if &rendered != s => {
                        vars.insert(k.clone(), Value::String(rendered));
                        changed = true;
                    }
                    Err(e) => log::warn!("could not render variable \"{}\": {}", k, e),
                    _ => {}
                }
            }
        }
    }

    for v in vars.values_mut() {
        if let Value::Object(sub) = v {
            changed = render_pass(sub, tera)? || changed;
        }
    }

    Ok(changed)
}

pub fn list_var_sets(var_set_path: &Path) -> anyhow::Result<Vec<String>> {
    let mut sets = Vec::new();
    for entry in std::fs::read_dir(var_set_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "yaml") {
            if let Some(stem) = path.file_stem() {
                sets.push(stem.to_string_lossy().to_string());
            }
        }
    }
    sets.sort();
    Ok(sets)
}

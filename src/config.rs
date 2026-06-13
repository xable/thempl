use std::path::PathBuf;

pub struct Config {
    pub templates_path: PathBuf,
    pub dest_path: PathBuf,
}

fn xdg_config_home() -> PathBuf {
    std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home::home_dir().unwrap().join(".config"))
}

fn config_root() -> PathBuf {
    xdg_config_home().join("thempl")
}

pub fn default_templates() -> PathBuf {
    config_root().join("templates")
}

pub fn default_var_sets() -> PathBuf {
    config_root().join("variable_sets")
}

pub fn default_ignores() -> PathBuf {
    config_root().join("ignores.yaml")
}

pub fn default_dest() -> PathBuf {
    home::home_dir().unwrap()
}

pub fn defaults_yaml() -> PathBuf {
    config_root().join("defaults.yaml")
}

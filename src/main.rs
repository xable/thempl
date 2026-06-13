mod config;
mod diff;
mod filters;
mod ignores;
mod renderer;
mod tui;
mod variables;

use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;

#[derive(Parser)]
#[command(name = "thempl", about = "A Jinja2 + YAML based config templater", version)]
struct Args {
    #[arg(short = 'l', help = "list variable sets")]
    list_var_sets: bool,

    #[arg(
        short = 't',
        default_value_t = config::default_templates().to_string_lossy().to_string(),
        help = "template directory"
    )]
    template_dir: String,

    #[arg(
        short = 'd',
        default_value_t = config::default_dest().to_string_lossy().to_string(),
        help = "destination directory"
    )]
    dest_dir: String,

    #[arg(
        short = 's',
        default_value_t = config::default_var_sets().to_string_lossy().to_string(),
        help = "variable set directory"
    )]
    var_set_dir: String,

    #[arg(
        short = 'i',
        default_value_t = config::default_ignores().to_string_lossy().to_string(),
        help = "ignores file"
    )]
    ignores_file: String,

    #[arg(short = 'e', help = "use environment variables")]
    env_vars: bool,

    #[arg(long = "diff", help = "show diff between rendered and existing files")]
    diff: bool,

    #[arg(long = "dry", help = "dry run")]
    dry: bool,

    #[arg(long = "tui", help = "interactive TUI mode")]
    tui: bool,

    #[arg(name = "VARIABLE_FILES", help = "additional variable files")]
    variable_files: Vec<String>,
}

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();

    let args = Args::parse();

    let templates_path = canonicalize_or(&args.template_dir)
        .context(format!("template directory not found: {}", args.template_dir))?;
    let dest_path = canonicalize_or(&args.dest_dir)
        .context(format!("destination directory not found: {}", args.dest_dir))?;
    let var_set_path = canonicalize_or(&args.var_set_dir).ok();
    let ignores_path = canonicalize_or(&args.ignores_file).ok();

    // ignores
    let ignores = match &ignores_path {
        Some(p) if p.exists() => ignores::IgnoreMatcher::load(p)?,
        _ => {
            log::warn!("ignores file not found, skipping");
            ignores::IgnoreMatcher::empty()
        }
    };

    // Tera
    let glob = templates_path.join("**/*").to_string_lossy().to_string();
    let mut tera = tera::Tera::new(&glob).context("failed to initialize Tera")?;
    tera.autoescape_on(Vec::<&str>::new());
    filters::register(&mut tera);

    let defaults_path = config::defaults_yaml();
    let var_set_dir_ref = var_set_path.as_ref().map(|p| p.as_path());
    let config = config::Config {
        templates_path,
        dest_path,
    };

    // --tui mode
    if args.tui {
        let var_sets = match var_set_dir_ref {
            Some(dir) if dir.exists() => {
                let names = variables::list_var_sets(dir)?;
                names
                    .into_iter()
                    .map(|name| tui::VarSetEntry {
                        name: name.clone(),
                        path: dir.join(format!("{}.yaml", name)),
                        enabled: false,
                    })
                    .collect()
            }
            _ => Vec::new(),
        };

        let mut app = tui::App::new(
            config,
            var_set_path,
            defaults_path,
            tera,
            ignores,
            var_sets,
        );
        return app.run();
    }

    // CLI modes (non-TUI)
    let cli_files: Vec<PathBuf> = args.variable_files.iter().map(PathBuf::from).collect();
    let vars = variables::load(
        Some(&defaults_path),
        var_set_dir_ref,
        &cli_files,
        args.env_vars,
        &mut tera,
    )?;

    let renderer = renderer::Renderer::new(config, tera, vars, ignores);

    if args.list_var_sets {
        match var_set_dir_ref {
            Some(dir) if dir.exists() => {
                for name in variables::list_var_sets(dir)? {
                    println!("{}", name);
                }
            }
            _ => anyhow::bail!("variable set directory not found or not specified"),
        }
    } else if args.diff {
        renderer.diff_all()?;
    } else if args.dry {
        renderer.dry_run()?;
    } else {
        renderer.render_and_write()?;
    }

    Ok(())
}

fn canonicalize_or(path: &str) -> anyhow::Result<PathBuf> {
    let p = PathBuf::from(shellexpand(path));
    if p.exists() {
        p.canonicalize()
            .context(format!("canonicalize: {}", p.display()))
    } else {
        Ok(p)
    }
}

fn shellexpand(s: &str) -> String {
    if s.starts_with('~') {
        if let Some(home) = home::home_dir() {
            return s.replacen('~', &home.to_string_lossy(), 1);
        }
    }
    s.to_string()
}

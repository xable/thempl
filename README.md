# thempl

**Jinja2 + YAML config templater** — render config files from templates using cascading YAML variables.

Inspired by [zenbu](https://github.com/metakirby5/zenbu).

## Features

- **Tera templates** (Jinja2-compatible) with variables, conditionals, loops, filters
- **Cascading variables**: env vars → `defaults.yaml` → variable set files (each overrides previous)
- **Interactive TUI**: browse variable sets, preview rendered files, view diffs
- **Built-in filters**: `nohash`, `to_rgb`, `to_chrome`, `to_apple`, `upper`, `lower`
- **Diff mode**: see what changed before writing
- **Dry run**: preview without writing

## Installation

```bash
cargo install --path .
```

Or copy the binary:

```bash
cp target/release/thempl ~/.local/bin/
```

## Quick start

```bash
# copy examples
cp -r examples/.config/thempl ~/.config/

# render all templates with defaults
thempl

# render with a variable set
thempl dracula

# show diff
thempl --diff

# dry run
thempl --dry

# interactive TUI
thempl --tui
```

## Configuration

Config root: `~/.config/thempl/`

```
~/.config/thempl/
├── defaults.yaml         # base variables
├── ignores.yaml          # regex patterns to skip templates
├── variable_sets/        # optional variable overrides
│   └── noku.yaml
└── templates/            # Jinja2 templates (directory structure preserved)
    └── .config/
        └── ...
```

### CLI

```
Usage: thempl [OPTIONS] [VARIABLE_FILES]...

Arguments:
  [VARIABLE_FILES]...  Additional variable files

Options:
  -l              List variable sets
  -t <DIR>        Template directory
  -d <DIR>        Destination directory
  -s <DIR>        Variable set directory
  -i <FILE>       Ignores file
  -e              Use environment variables
  --diff          Show diff between rendered and existing files
  --dry           Dry run (preview only)
  --tui           Interactive TUI mode
```

### TUI controls

| Key | Var Sets panel | Files panel |
|---|---|---|
| `Tab` | Switch to Files | Switch to Var Sets |
| `↑`/`↓` / `k`/`j` | Navigate | Navigate |
| `Space` | Toggle variable set | Toggle file selection |
| `Enter` | Toggle variable set | Open file preview |
| `R` | Render selected | Render selected |
| `D` | Toggle diff mode | Toggle diff mode |
| `Q` / `Esc` | Quit | Quit |

## Built-in filters

| Filter | Input | Output |
|---|---|---|
| `nohash` | `"#ff0000"` | `"ff0000"` |
| `to_rgb` | `"#ff0000"` | `"(255,0,0)"` |
| `to_chrome` | `"#ff0000"` | `"[255, 0, 0]"` |
| `to_apple` | `"#ff0000"` | `(red: 1.0, green: 0.0, blue: 0.0)` |
| `upper` | `"hello"` | `"HELLO"` |
| `lower` | `"HELLO"` | `"hello"` |

## License

MIT

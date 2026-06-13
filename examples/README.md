# thempl examples

## Structure

```
.config/thempl/
├── defaults.yaml           # base variables (fonts, colors, padding)
├── ignores.yaml            # regex patterns for files to skip
├── variable_sets/
│   ├── dracula.yaml        # dark theme (overrides defaults)
│   └── light.yaml          # light theme
└── templates/
    ├── .Xresources         # for loops, conditionals, filters
    ├── alacritty.toml      # direct variable access
    ├── polybar/config.ini  # to_rgb, nohash filters
    └── waybar/style.css    # loop over a string array
```

## Usage

```bash
# copy examples to ~/.config
cp -r .config/thempl ~/.config/

# render with the default theme (tokyo-night)
thempl

# render with dracula
thempl dracula

# render with light theme
thempl light

# show diff
thempl --diff

# dry run
thempl --dry

# interactive TUI
thempl --tui
```

## Template features

| Construct | Example | Result |
|---|---|---|
| `{{ var }}` | `{{ theme.background }}` | `#1a1b26` |
| `{{ var[key] }}` | `{{ theme["blue"] }}` | `#7aa2f7` |
| `{% for x in list %}` | `{% for f in term_fonts %}` | iterates fonts |
| `{% if cond %}` | `{% if use_bold %}` | conditional render |
| `\| filter` | `{{ "##ff0000" \| nohash }}` | `ff0000` |
| `\| to_rgb` | `{{ theme.blue \| to_rgb }}` | `(122,162,247)` |
| `\| to_chrome` | `{{ theme.blue \| to_chrome }}` | `[122, 162, 247]` |
| `{{ arr[0] }}` | `{{ term_fonts[0] }}` | `JetBrains Mono` |

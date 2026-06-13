use std::path::PathBuf;

use anyhow::Context;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use serde_json::Value;
use similar::{ChangeTag, TextDiff};

use crate::config::Config;
use crate::{filters, ignores, variables};

#[derive(PartialEq)]
pub enum Focus {
    VarSets,
    Files,
}

pub enum PreviewMode {
    Compact,
    Detail(usize),
}

pub struct VarSetEntry {
    pub name: String,
    pub path: PathBuf,
    pub enabled: bool,
}

pub struct RenderedFile {
    pub name: String,
    pub ok: bool,
    pub error: String,
    pub content: String,
    pub selected: bool,
}

pub struct App {
    pub config: Config,
    pub var_set_dir: Option<PathBuf>,
    pub defaults_path: PathBuf,
    pub tera: tera::Tera,
    pub ignores: ignores::IgnoreMatcher,
    pub var_sets: Vec<VarSetEntry>,
    pub list_state: ListState,
    pub file_list_state: ListState,
    pub detail_scroll: usize,
    pub detail_index: usize,
    pub rendered_files: Vec<RenderedFile>,
    pub diff_content: String,
    pub status: String,
    pub show_diff: bool,
    pub should_quit: bool,
    pub focus: Focus,
    pub mode: PreviewMode,
}

impl App {
    pub fn new(
        config: Config,
        var_set_dir: Option<PathBuf>,
        defaults_path: PathBuf,
        tera: tera::Tera,
        ignores: ignores::IgnoreMatcher,
        var_sets: Vec<VarSetEntry>,
    ) -> Self {
        let mut list_state = ListState::default();
        if !var_sets.is_empty() {
            list_state.select(Some(0));
        }
        Self {
            config,
            var_set_dir,
            defaults_path,
            tera,
            ignores,
            var_sets,
            list_state,
            file_list_state: ListState::default(),
            detail_scroll: 0,
            detail_index: 0,
            rendered_files: Vec::new(),
            diff_content: String::new(),
            status: "Ready".to_string(),
            show_diff: false,
            should_quit: false,
            focus: Focus::VarSets,
            mode: PreviewMode::Compact,
        }
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        crossterm::terminal::enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        use crossterm::ExecutableCommand;
        stdout.execute(crossterm::terminal::EnterAlternateScreen)?;

        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(stdout))?;
        terminal.clear()?;

        self.update_preview()?;

        while !self.should_quit {
            terminal.draw(|f| self.draw(f))?;

            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    self.handle_key(key.code);
                }
            }
        }

        let mut stdout = std::io::stdout();
        stdout.execute(crossterm::terminal::LeaveAlternateScreen)?;
        crossterm::terminal::disable_raw_mode()?;
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let chunks =
            Layout::vertical([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])
                .split(area);

        frame.render_widget(
            Line::from(Span::styled(
                " thempl ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            chunks[0],
        );

        let main =
            Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)])
                .split(chunks[1]);
        self.draw_var_set_list(frame, main[0]);
        match &self.mode {
            PreviewMode::Compact => self.draw_compact(frame, main[1]),
            PreviewMode::Detail(i) => self.draw_detail(frame, main[1], *i),
        }

        let mode_label = if self.show_diff { " DIFF" } else { " PREVIEW" };
        let var_count = self.var_sets.iter().filter(|v| v.enabled).count();
        let hint = match (&self.mode, &self.focus) {
            (PreviewMode::Detail(_), _) => " [↑/↓] scroll  [Enter/Esc] back  [Q] Quit",
            (PreviewMode::Compact, Focus::VarSets) => {
                " [Space] Toggle  [Tab] Files  [R] Render  [D] Diff  [Q] Quit"
            }
            (PreviewMode::Compact, Focus::Files) => {
                " [Space] Select  [Enter] Open  [Tab] VarSets  [R] Render  [D] Diff  [Q] Quit"
            }
        };

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(mode_label, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(format!("  [{}/{}] ", var_count, self.var_sets.len())),
                Span::styled(&self.status, Style::default().fg(Color::DarkGray)),
                Span::raw("  "),
                Span::styled(hint, Style::default().fg(Color::DarkGray)),
            ])),
            chunks[2],
        );
    }

    fn draw_var_set_list(&mut self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .var_sets
            .iter()
            .map(|v| {
                let prefix = if v.enabled { "[x]" } else { "[ ]" };
                let style = if v.enabled {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(Span::styled(
                    format!(" {} {}", prefix, v.name),
                    style,
                )))
            })
            .collect();

        let border_style = if self.focus == Focus::VarSets {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Cyan)
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .title(" Var Sets ")
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            );

        frame.render_stateful_widget(list, area, &mut self.list_state);
    }

    fn draw_compact(&mut self, frame: &mut Frame, area: Rect) {
        let border_style = if self.focus == Focus::Files {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Cyan)
        };

        let items: Vec<ListItem> = self
            .rendered_files
            .iter()
            .map(|f| {
                let (sel_prefix, sel_style) = if f.selected {
                    ("[x]", Style::default().fg(Color::Green))
                } else {
                    ("[ ]", Style::default().fg(Color::DarkGray))
                };
                let name_style = if f.ok {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Red)
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{} ", sel_prefix), sel_style),
                    Span::styled(&f.name, name_style),
                ]))
            })
            .collect();

        if items.is_empty() {
            let block = Block::default()
                .title(" Files ")
                .borders(Borders::ALL)
                .border_style(border_style);
            frame.render_widget(Paragraph::new("(no templates)").block(block), area);
            return;
        }

        let list = List::new(items)
            .block(
                Block::default()
                    .title(" Files ")
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            );

        frame.render_stateful_widget(list, area, &mut self.file_list_state);
    }

    fn draw_detail(&mut self, frame: &mut Frame, area: Rect, index: usize) {
        let file = &self.rendered_files[index];

        let total = file.content.lines().count();
        let max_scroll = total.saturating_sub(1);
        self.detail_scroll = self.detail_scroll.min(max_scroll);
        let scroll_pos = self.detail_scroll + 1;
        let scroll_pct = format!(" {:>2}/{} ", scroll_pos.min(total), total);

        let block = Block::default()
            .title(format!(" {} ", file.name))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title_bottom(Line::from(Span::styled(
                format!("{}[↑/↓] scroll  [Enter/Esc] back", scroll_pct),
                Style::default().fg(Color::DarkGray),
            )));

        if !file.ok {
            frame.render_widget(
                Paragraph::new(Text::from(Line::from(Span::styled(
                    format!(" ERROR: {}", file.error),
                    Style::default().fg(Color::Red),
                ))))
                .block(block),
                area,
            );
            return;
        }

        let text: Text = file
            .content
            .lines()
            .map(|line| Line::from(Span::raw(line.to_string())))
            .collect();
        frame.render_widget(
            Paragraph::new(text)
                .block(block)
                .scroll((self.detail_scroll as u16, 0)),
            area,
        );
    }

    fn handle_key(&mut self, code: KeyCode) {
        match &self.mode {
            PreviewMode::Detail(_) => match code {
                KeyCode::Enter | KeyCode::Esc => {
                    self.mode = PreviewMode::Compact;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.detail_scroll = self.detail_scroll.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let max = self
                        .rendered_files
                        .get(self.detail_index)
                        .map(|f| f.content.lines().count().saturating_sub(1))
                        .unwrap_or(0);
                    if self.detail_scroll < max {
                        self.detail_scroll += 1;
                    }
                }
                KeyCode::PageUp => {
                    self.detail_scroll = self.detail_scroll.saturating_sub(20);
                }
                KeyCode::PageDown => {
                    let max = self
                        .rendered_files
                        .get(self.detail_index)
                        .map(|f| f.content.lines().count().saturating_sub(1))
                        .unwrap_or(0);
                    self.detail_scroll = (self.detail_scroll + 20).min(max);
                }
                KeyCode::Home | KeyCode::Char('g') => {
                    self.detail_scroll = 0;
                }
                KeyCode::End | KeyCode::Char('G') => {
                    let max = self
                        .rendered_files
                        .get(self.detail_index)
                        .map(|f| f.content.lines().count().saturating_sub(1))
                        .unwrap_or(0);
                    self.detail_scroll = max;
                }
                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    self.should_quit = true;
                }
                _ => {}
            },
            PreviewMode::Compact => match code {
                KeyCode::Tab => {
                    self.focus = if self.focus == Focus::VarSets {
                        Focus::Files
                    } else {
                        Focus::VarSets
                    };
                    if self.focus == Focus::Files
                        && self.file_list_state.selected().is_none()
                        && !self.rendered_files.is_empty()
                    {
                        self.file_list_state.select(Some(0));
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => match self.focus {
                    Focus::VarSets => {
                        let i = self.list_state.selected().unwrap_or(0);
                        if i > 0 {
                            self.list_state.select(Some(i - 1));
                        }
                    }
                    Focus::Files => {
                        let i = self.file_list_state.selected().unwrap_or(0);
                        if i > 0 {
                            self.file_list_state.select(Some(i - 1));
                        }
                    }
                },
                KeyCode::Down | KeyCode::Char('j') => match self.focus {
                    Focus::VarSets => {
                        let i = self.list_state.selected().unwrap_or(0);
                        if i + 1 < self.var_sets.len() {
                            self.list_state.select(Some(i + 1));
                        }
                    }
                    Focus::Files => {
                        let i = self.file_list_state.selected().unwrap_or(0);
                        if i + 1 < self.rendered_files.len() {
                            self.file_list_state.select(Some(i + 1));
                        }
                    }
                },
                KeyCode::Enter => match self.focus {
                    Focus::VarSets => {
                        if let Some(i) = self.list_state.selected() {
                            if i < self.var_sets.len() {
                                self.var_sets[i].enabled = !self.var_sets[i].enabled;
                                let _ = self.update_preview();
                            }
                        }
                    }
                    Focus::Files => {
                        if let Some(i) = self.file_list_state.selected() {
                            if i < self.rendered_files.len() {
                                self.detail_scroll = 0;
                                self.detail_index = i;
                                self.mode = PreviewMode::Detail(i);
                            }
                        }
                    }
                },
                KeyCode::Char(' ') => match self.focus {
                    Focus::VarSets => {
                        if let Some(i) = self.list_state.selected() {
                            if i < self.var_sets.len() {
                                self.var_sets[i].enabled = !self.var_sets[i].enabled;
                                let _ = self.update_preview();
                            }
                        }
                    }
                    Focus::Files => {
                        if let Some(i) = self.file_list_state.selected() {
                            if i < self.rendered_files.len() {
                                self.rendered_files[i].selected = !self.rendered_files[i].selected;
                            }
                        }
                    }
                },
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    self.show_diff = false;
                    match self.do_render() {
                        Ok(count) => {
                            self.status = format!("Rendered {} file(s)!", count);
                            let _ = self.update_preview();
                        }
                        Err(e) => {
                            self.status = format!("Error: {}", e);
                        }
                    }
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    self.show_diff = !self.show_diff;
                    if self.show_diff && self.diff_content.is_empty() {
                        let _ = self.update_diff();
                    }
                    if !self.show_diff {
                        let _ = self.update_preview();
                    }
                    self.status = if self.show_diff {
                        "Diff mode".to_string()
                    } else {
                        "Preview mode".to_string()
                    };
                }
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    self.should_quit = true;
                }
                _ => {}
            },
        }
    }

    fn load_vars(&mut self) -> anyhow::Result<Value> {
        let selected: Vec<PathBuf> = self
            .var_sets
            .iter()
            .filter(|v| v.enabled)
            .map(|v| v.path.clone())
            .collect();

        let vars = variables::load(
            Some(&self.defaults_path),
            self.var_set_dir.as_ref().map(|p| p.as_path()),
            &selected,
            false,
            &mut self.tera,
        )?;
        Ok(Value::Object(vars))
    }

    fn update_preview(&mut self) -> anyhow::Result<()> {
        let vars = self.load_vars()?;
        let ctx =
            tera::Context::from_serialize(&vars).context("failed to build Tera context")?;

        let mut files = Vec::new();
        for entry in walkdir::WalkDir::new(&self.config.templates_path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| !self.ignores.should_ignore(e.path()))
        {
            let relative = entry
                .path()
                .strip_prefix(&self.config.templates_path)
                .unwrap();
            let relative_str = relative.to_string_lossy().replace('\\', "/");

            // preserve selection state across refreshes
            let prev_selected = self
                .rendered_files
                .iter()
                .find(|f| f.name == relative_str)
                .map(|f| f.selected)
                .unwrap_or(true);

            match self.tera.render(&relative_str, &ctx) {
                Ok(rendered) => {
                    files.push(RenderedFile {
                        name: relative.to_string_lossy().to_string(),
                        ok: true,
                        error: String::new(),
                        content: rendered,
                        selected: prev_selected,
                    });
                }
                Err(e) => {
                    files.push(RenderedFile {
                        name: relative.to_string_lossy().to_string(),
                        ok: false,
                        error: format!("{}", e),
                        content: String::new(),
                        selected: prev_selected,
                    });
                }
            }
        }

        self.rendered_files = files;

        if self.focus == Focus::Files
            && self.file_list_state.selected().is_none()
            && !self.rendered_files.is_empty()
        {
            self.file_list_state.select(Some(0));
        }

        self.status = format!(
            "{} set(s) active",
            self.var_sets.iter().filter(|v| v.enabled).count()
        );
        Ok(())
    }

    fn update_diff(&mut self) -> anyhow::Result<()> {
        let vars = self.load_vars()?;
        let ctx =
            tera::Context::from_serialize(&vars).context("failed to build Tera context")?;

        let mut output = String::new();
        for entry in walkdir::WalkDir::new(&self.config.templates_path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| !self.ignores.should_ignore(e.path()))
        {
            let relative = entry
                .path()
                .strip_prefix(&self.config.templates_path)
                .unwrap();
            let relative_str = relative.to_string_lossy().replace('\\', "/");
            match self.tera.render(&relative_str, &ctx) {
                Ok(rendered) => {
                    let dest = self.config.dest_path.join(relative);
                    let dest_str = dest.to_string_lossy().to_string();
                    let dest_content = std::fs::read_to_string(&dest).unwrap_or_default();
                    let text_diff = TextDiff::from_lines(&dest_content, &rendered);

                    output.push_str(&format!("--- {}\n+++ {} (rendered)\n", dest_str, dest_str));
                    for change in text_diff.iter_all_changes() {
                        let line = match change.tag() {
                            ChangeTag::Equal => format!(" {}", change.value()),
                            ChangeTag::Insert => format!("+{}", change.value()),
                            ChangeTag::Delete => format!("-{}", change.value()),
                        };
                        output.push_str(&line);
                    }
                }
                Err(e) => {
                    output.push_str(&format!("── {} ──\n  [ERROR: {}]\n", relative.display(), e));
                }
            }
        }
        if output.is_empty() {
            output = "(no templates found)".to_string();
        }
        self.diff_content = output;
        Ok(())
    }

    fn do_render(&mut self) -> anyhow::Result<usize> {
        let vars = self.load_vars()?;
        let vars_map = match vars {
            Value::Object(m) => m,
            _ => unreachable!(),
        };
        let ctx = tera::Context::from_serialize(&Value::Object(vars_map))
            .context("failed to build Tera context")?;

        let glob = self
            .config
            .templates_path
            .join("**/*")
            .to_string_lossy()
            .to_string();
        let mut tera = tera::Tera::new(&glob).context("failed to init Tera")?;
        tera.autoescape_on(Vec::<&str>::new());
        filters::register(&mut tera);

        let mut count = 0;
        for rf in &self.rendered_files {
            if !rf.selected {
                continue;
            }
            let relative = rf.name.replace('\\', "/");
            match tera.render(&relative, &ctx) {
                Ok(rendered) => {
                    let dest = self.config.dest_path.join(&rf.name);
                    if let Some(parent) = dest.parent() {
                        std::fs::create_dir_all(parent)
                            .context(format!("failed to create {}", parent.display()))?;
                    }
                    std::fs::write(&dest, &rendered)
                        .context(format!("failed to write {}", dest.display()))?;
                    log::info!("rendered {}", dest.display());
                    count += 1;
                }
                Err(e) => {
                    self.status = format!("Error rendering {}: {}", rf.name, e);
                }
            }
        }
        Ok(count)
    }
}

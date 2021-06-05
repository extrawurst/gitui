use crate::{
    components::{
        visibility_blocking, CommandBlocking, CommandInfo, Component,
        DrawableComponent, EventState,
    },
    keys::SharedKeyConfig,
    strings,
    ui::{self, style::SharedTheme},
};
use anyhow::{anyhow, bail, Result};
use asyncgit::{
    sync::{get_config_string, utils::repo_work_dir},
    CWD,
};
use crossterm::{
    event::Event,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use scopeguard::defer;
use std::ffi::OsStr;
use std::{env, io, path::Path, process::Command};
use tui::{
    backend::Backend,
    layout::Rect,
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

///
pub struct ExternalEditorComponent {
    visible: bool,
    theme: SharedTheme,
    key_config: SharedKeyConfig,
}

impl ExternalEditorComponent {
    ///
    pub fn new(
        theme: SharedTheme,
        key_config: SharedKeyConfig,
    ) -> Self {
        Self {
            visible: false,
            theme,
            key_config,
        }
    }

    /// opens file at given `path` in an available editor
    pub fn open_file_in_editor(path: &Path) -> Result<()> {
        let work_dir = repo_work_dir(CWD)?;

        let path = if path.is_relative() {
            Path::new(&work_dir).join(path)
        } else {
            path.into()
        };

        if !path.exists() {
            bail!("file not found: {:?}", path);
        }

        io::stdout().execute(LeaveAlternateScreen)?;
        defer! {
            io::stdout().execute(EnterAlternateScreen).expect("reset terminal");
        }

        let environment_options = ["GIT_EDITOR", "VISUAL", "EDITOR"];

        let editor = env::var(environment_options[0])
            .ok()
            .or_else(|| get_config_string(CWD, "core.editor").ok()?)
            .or_else(|| env::var(environment_options[1]).ok())
            .or_else(|| env::var(environment_options[2]).ok())
            .unwrap_or_else(|| String::from("vi"));

        // TODO: proper handling arguments containing whitespaces
        // This does not do the right thing if the input is `editor --something "with spaces"`

        // deal with "editor name with spaces" p1 p2 p3
        // and with "editor_no_spaces" p1 p2 p3
        // does not address spaces in pn
        let mut echars = editor.chars().peekable();

        let first_char = *echars.peek().ok_or_else(|| {
            anyhow!(
                "editor env variable found empty: {}",
                environment_options.join(" or ")
            )
        })?;
        let command: String = if first_char == '\"' {
            echars
                .by_ref()
                .skip(1)
                .take_while(|c| *c != '\"')
                .collect()
        } else {
            echars.by_ref().take_while(|c| *c != ' ').collect()
        };

        let remainder_str = echars.collect::<String>();
        let remainder = remainder_str.split_whitespace();

        let mut args: Vec<&OsStr> =
            remainder.map(|s| OsStr::new(s)).collect();

        args.push(path.as_os_str());

        Command::new(command.clone())
            .current_dir(work_dir)
            .args(args)
            .status()
            .map_err(|e| anyhow!("\"{}\": {}", command, e))?;

        Ok(())
    }
}

impl DrawableComponent for ExternalEditorComponent {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        _rect: Rect,
    ) -> Result<()> {
        if self.visible {
            let txt = Spans::from(
                strings::msg_opening_editor(&self.key_config)
                    .split('\n')
                    .map(|string| {
                        Span::raw::<String>(string.to_string())
                    })
                    .collect::<Vec<Span>>(),
            );

            let area = ui::centered_rect_absolute(25, 3, f.size());
            f.render_widget(Clear, area);
            f.render_widget(
                Paragraph::new(txt)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Thick)
                            .border_style(self.theme.block(true)),
                    )
                    .style(self.theme.block(true)),
                area,
            );
        }

        Ok(())
    }
}

impl Component for ExternalEditorComponent {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        _force_all: bool,
    ) -> CommandBlocking {
        if self.visible {
            out.clear();
        }

        visibility_blocking(self)
    }

    fn event(&mut self, _ev: Event) -> Result<EventState> {
        if self.visible {
            return Ok(EventState::Consumed);
        }

        Ok(EventState::NotConsumed)
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn hide(&mut self) {
        self.visible = false;
    }

    fn show(&mut self) -> Result<()> {
        self.visible = true;

        Ok(())
    }
}

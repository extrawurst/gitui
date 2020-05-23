mod utils;

use crate::{
    components::{
        CommandBlocking, CommandInfo, Component, DrawableComponent,
        ScrollType,
    },
    keys,
    strings::{self, commands},
    ui::calc_scroll_top,
    ui::style::Theme,
};
use asyncgit::{sync, AsyncLog, AsyncNotification, FetchStatus, CWD};
use crossbeam_channel::Sender;
use crossterm::event::Event;
use std::{borrow::Cow, cmp, convert::TryFrom, time::Instant};
use sync::Tags;
use tui::{
    backend::Backend,
    layout::{Alignment, Rect},
    widgets::{Block, Borders, Paragraph, Text},
    Frame,
};
use utils::{ItemBatch, LogEntry};

static SLICE_SIZE: usize = 1200;
///
pub struct Revlog {
    selection: usize,
    count_total: usize,
    items: ItemBatch,
    git_log: AsyncLog,
    visible: bool,
    scroll_state: (Instant, f32),
    tags: Tags,
    current_size: (u16, u16),
    scroll_top: usize,
    theme: Theme,
}

impl Revlog {
    ///
    pub fn new(
        sender: &Sender<AsyncNotification>,
        theme: &Theme,
    ) -> Self {
        Self {
            items: ItemBatch::default(),
            git_log: AsyncLog::new(sender.clone()),
            selection: 0,
            count_total: 0,
            visible: false,
            scroll_state: (Instant::now(), 0_f32),
            tags: Tags::new(),
            current_size: (0, 0),
            scroll_top: 0,
            theme: *theme,
        }
    }

    ///
    pub fn any_work_pending(&self) -> bool {
        self.git_log.is_pending()
    }

    fn selection_max(&self) -> usize {
        self.count_total.saturating_sub(1)
    }

    ///
    pub fn update(&mut self) {
        if self.visible {
            let log_changed =
                self.git_log.fetch().unwrap() == FetchStatus::Started;

            self.count_total = self.git_log.count().unwrap();

            if self
                .items
                .needs_data(self.selection, self.selection_max())
                || log_changed
            {
                self.fetch_commits();
            }

            if self.tags.is_empty() {
                self.tags = sync::get_tags(CWD).unwrap();
            }
        }
    }

    fn fetch_commits(&mut self) {
        let want_min = self.selection.saturating_sub(SLICE_SIZE / 2);

        let commits = sync::get_commits_info(
            CWD,
            &self.git_log.get_slice(want_min, SLICE_SIZE).unwrap(),
            self.current_size.0.into(),
        );

        if let Ok(commits) = commits {
            self.items.set_items(want_min, commits);
        }
    }

    fn move_selection(&mut self, scroll: ScrollType) {
        self.update_scroll_speed();

        #[allow(clippy::cast_possible_truncation)]
        let speed_int = usize::try_from(self.scroll_state.1 as i64)
            .unwrap()
            .max(1);

        let page_offset =
            usize::from(self.current_size.1).saturating_sub(1);

        self.selection = match scroll {
            ScrollType::Up => {
                self.selection.saturating_sub(speed_int)
            }
            ScrollType::Down => {
                self.selection.saturating_add(speed_int)
            }
            ScrollType::PageUp => {
                self.selection.saturating_sub(page_offset)
            }
            ScrollType::PageDown => {
                self.selection.saturating_add(page_offset)
            }
            ScrollType::Home => 0,
            ScrollType::End => self.selection_max(),
        };

        self.selection =
            cmp::min(self.selection, self.selection_max());

        self.update();
    }

    fn update_scroll_speed(&mut self) {
        const REPEATED_SCROLL_THRESHOLD_MILLIS: u128 = 300;
        const SCROLL_SPEED_START: f32 = 0.1_f32;
        const SCROLL_SPEED_MAX: f32 = 10_f32;
        const SCROLL_SPEED_MULTIPLIER: f32 = 1.05_f32;

        let now = Instant::now();

        let since_last_scroll =
            now.duration_since(self.scroll_state.0);

        self.scroll_state.0 = now;

        let speed = if since_last_scroll.as_millis()
            < REPEATED_SCROLL_THRESHOLD_MILLIS
        {
            self.scroll_state.1 * SCROLL_SPEED_MULTIPLIER
        } else {
            SCROLL_SPEED_START
        };

        self.scroll_state.1 = speed.min(SCROLL_SPEED_MAX);
    }

    fn add_entry<'a>(
        e: &'a LogEntry,
        selected: bool,
        txt: &mut Vec<Text<'a>>,
        tags: Option<String>,
        theme: &Theme,
    ) {
        const ELEMENTS_PER_LINE: usize = 10;

        txt.reserve(ELEMENTS_PER_LINE);

        let splitter_txt = Cow::from(" ");
        let splitter =
            Text::Styled(splitter_txt, theme.text(true, selected));

        txt.push(Text::Styled(
            Cow::from(&e.hash[0..7]),
            theme.commit_hash(selected),
        ));
        txt.push(splitter.clone());
        txt.push(Text::Styled(
            Cow::from(e.time.as_str()),
            theme.commit_time(selected),
        ));
        txt.push(splitter.clone());
        txt.push(Text::Styled(
            Cow::from(e.author.as_str()),
            theme.commit_author(selected),
        ));
        txt.push(splitter.clone());
        txt.push(Text::Styled(
            Cow::from(if let Some(tags) = tags {
                format!(" {}", tags)
            } else {
                String::from("")
            }),
            theme.tab(true).bg(theme.text(true, selected).bg),
        ));
        txt.push(splitter);
        txt.push(Text::Styled(
            Cow::from(e.msg.as_str()),
            theme.text(true, selected),
        ));
        txt.push(Text::Raw(Cow::from("\n")));
    }

    fn get_text(&self, height: usize) -> Vec<Text> {
        let selection = self.relative_selection();

        let mut txt = Vec::new();

        for (idx, e) in self
            .items
            .items
            .iter()
            .skip(self.scroll_top)
            .take(height)
            .enumerate()
        {
            let tag = if let Some(tags) = self.tags.get(&e.hash) {
                Some(tags.join(" "))
            } else {
                None
            };
            Self::add_entry(
                e,
                idx + self.scroll_top == selection,
                &mut txt,
                tag,
                &self.theme,
            );
        }

        txt
    }

    fn relative_selection(&self) -> usize {
        self.selection.saturating_sub(self.items.index_offset)
    }
}

impl DrawableComponent for Revlog {
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        self.current_size = (
            area.width.saturating_sub(2),
            area.height.saturating_sub(2),
        );

        let height_in_lines = self.current_size.1 as usize;
        let selection = self.relative_selection();

        self.scroll_top = calc_scroll_top(
            self.scroll_top,
            height_in_lines,
            selection,
        );

        let title = format!(
            "{} {}/{}",
            strings::LOG_TITLE,
            self.count_total.saturating_sub(self.selection),
            self.count_total,
        );

        f.render_widget(
            Paragraph::new(self.get_text(height_in_lines).iter())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(title.as_str())
                        .border_style(self.theme.block(true))
                        .title_style(self.theme.title(true)),
                )
                .alignment(Alignment::Left),
            area,
        );
    }
}

impl Component for Revlog {
    fn event(&mut self, ev: Event) -> bool {
        if self.visible {
            if let Event::Key(k) = ev {
                return match k {
                    keys::MOVE_UP => {
                        self.move_selection(ScrollType::Up);
                        true
                    }
                    keys::MOVE_DOWN => {
                        self.move_selection(ScrollType::Down);
                        true
                    }
                    keys::SHIFT_UP | keys::HOME => {
                        self.move_selection(ScrollType::Home);
                        true
                    }
                    keys::SHIFT_DOWN | keys::END => {
                        self.move_selection(ScrollType::End);
                        true
                    }
                    keys::PAGE_UP => {
                        self.move_selection(ScrollType::PageUp);
                        true
                    }
                    keys::PAGE_DOWN => {
                        self.move_selection(ScrollType::PageDown);
                        true
                    }
                    _ => false,
                };
            }
        }

        false
    }

    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        out.push(CommandInfo::new(
            commands::SCROLL,
            self.visible,
            self.visible || force_all,
        ));

        if self.visible {
            CommandBlocking::Blocking
        } else {
            CommandBlocking::PassingOn
        }
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn hide(&mut self) {
        self.visible = false;
        self.git_log.set_background();
    }

    fn show(&mut self) {
        self.visible = true;
        self.git_log.fetch().unwrap();
    }
}

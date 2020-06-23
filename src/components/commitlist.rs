use super::utils::logitems::{ItemBatch, LogEntry};
use crate::{
    components::{
        CommandBlocking, CommandInfo, Component, DrawableComponent,
        ScrollType,
    },
    keys,
    strings::commands,
    ui::calc_scroll_top,
    ui::style::{SharedTheme, Theme},
};
use anyhow::Result;
use asyncgit::sync;
use crossterm::event::Event;
use std::{
    borrow::Cow, cell::Cell, cmp, convert::TryFrom, time::Instant,
};
use sync::Tags;
use tui::{
    backend::Backend,
    layout::{Alignment, Rect},
    widgets::{Block, Borders, Paragraph, Text},
    Frame,
};

const ELEMENTS_PER_LINE: usize = 10;

///
pub struct CommitList {
    title: String,
    selection: usize,
    branch: Option<String>,
    count_total: usize,
    items: ItemBatch,
    scroll_state: (Instant, f32),
    tags: Option<Tags>,
    current_size: Cell<(u16, u16)>,
    scroll_top: Cell<usize>,
    theme: SharedTheme,
}

impl CommitList {
    ///
    pub fn new(title: &str, theme: SharedTheme) -> Self {
        Self {
            items: ItemBatch::default(),
            selection: 0,
            branch: None,
            count_total: 0,
            scroll_state: (Instant::now(), 0_f32),
            tags: None,
            current_size: Cell::new((0, 0)),
            scroll_top: Cell::new(0),
            theme,
            title: String::from(title),
        }
    }

    ///
    pub fn items(&mut self) -> &mut ItemBatch {
        &mut self.items
    }

    ///
    pub fn set_branch(&mut self, name: Option<String>) {
        self.branch = name;
    }

    ///
    pub const fn selection(&self) -> usize {
        self.selection
    }

    ///
    pub fn current_size(&self) -> (u16, u16) {
        self.current_size.get()
    }

    ///
    pub fn set_count_total(&mut self, total: usize) {
        self.count_total = total;
    }

    ///
    #[allow(clippy::missing_const_for_fn)]
    pub fn selection_max(&self) -> usize {
        self.count_total.saturating_sub(1)
    }

    ///
    pub fn tags(&self) -> Option<&Tags> {
        self.tags.as_ref()
    }

    ///
    pub fn has_tags(&self) -> bool {
        self.tags.is_some()
    }

    ///
    pub fn clear(&mut self) {
        self.tags = None;
        self.items.clear();
    }

    ///
    pub fn set_tags(&mut self, tags: Tags) {
        self.tags = Some(tags);
    }

    ///
    pub fn selected_entry(&self) -> Option<&LogEntry> {
        self.items.iter().nth(
            self.selection.saturating_sub(self.items.index_offset()),
        )
    }

    fn move_selection(&mut self, scroll: ScrollType) -> Result<bool> {
        self.update_scroll_speed();

        #[allow(clippy::cast_possible_truncation)]
        let speed_int =
            usize::try_from(self.scroll_state.1 as i64)?.max(1);

        let page_offset =
            usize::from(self.current_size.get().1).saturating_sub(1);

        let new_selection = match scroll {
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

        let new_selection =
            cmp::min(new_selection, self.selection_max());

        let needs_update = new_selection != self.selection;

        self.selection = new_selection;

        Ok(needs_update)
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
        txt.reserve(ELEMENTS_PER_LINE);

        let splitter_txt = Cow::from(" ");
        let splitter =
            Text::Styled(splitter_txt, theme.text(true, selected));

        // commit hash
        txt.push(Text::Styled(
            Cow::from(e.hash_short.as_str()),
            theme.commit_hash(selected),
        ));

        txt.push(splitter.clone());

        // commit timestamp
        txt.push(Text::Styled(
            Cow::from(e.time.as_str()),
            theme.commit_time(selected),
        ));

        txt.push(splitter.clone());

        // commit author
        txt.push(Text::Styled(
            Cow::from(e.author.as_str()),
            theme.commit_author(selected),
        ));

        txt.push(splitter.clone());

        // commit tags
        txt.push(Text::Styled(
            Cow::from(if let Some(tags) = tags {
                format!(" {}", tags)
            } else {
                String::from("")
            }),
            theme.tags(selected),
        ));

        txt.push(splitter);

        // commit msg
        txt.push(Text::Styled(
            Cow::from(e.msg.as_str()),
            theme.text(true, selected),
        ));
        txt.push(Text::Raw(Cow::from("\n")));
    }

    fn get_text(&self, height: usize) -> Vec<Text> {
        let selection = self.relative_selection();

        let mut txt = Vec::with_capacity(height * ELEMENTS_PER_LINE);

        for (idx, e) in self
            .items
            .iter()
            .skip(self.scroll_top.get())
            .take(height)
            .enumerate()
        {
            let tags = if let Some(tags) =
                self.tags.as_ref().and_then(|t| t.get(&e.id))
            {
                Some(tags.join(" "))
            } else {
                None
            };

            Self::add_entry(
                e,
                idx + self.scroll_top.get() == selection,
                &mut txt,
                tags,
                &self.theme,
            );
        }

        txt
    }

    #[allow(clippy::missing_const_for_fn)]
    fn relative_selection(&self) -> usize {
        self.selection.saturating_sub(self.items.index_offset())
    }
}

impl DrawableComponent for CommitList {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        area: Rect,
    ) -> Result<()> {
        self.current_size.set((
            area.width.saturating_sub(2),
            area.height.saturating_sub(2),
        ));

        let height_in_lines = self.current_size.get().1 as usize;
        let selection = self.relative_selection();

        self.scroll_top.set(calc_scroll_top(
            self.scroll_top.get(),
            height_in_lines,
            selection,
        ));

        let branch_post_fix =
            self.branch.as_ref().map(|b| format!("- {{{}}}", b));

        let title = format!(
            "{} {}/{} {}",
            self.title,
            self.count_total.saturating_sub(self.selection),
            self.count_total,
            branch_post_fix.as_deref().unwrap_or(""),
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

        Ok(())
    }
}

impl Component for CommitList {
    fn event(&mut self, ev: Event) -> Result<bool> {
        if let Event::Key(k) = ev {
            let selection_changed = match k {
                keys::MOVE_UP => {
                    self.move_selection(ScrollType::Up)?
                }
                keys::MOVE_DOWN => {
                    self.move_selection(ScrollType::Down)?
                }
                keys::SHIFT_UP | keys::HOME => {
                    self.move_selection(ScrollType::Home)?
                }
                keys::SHIFT_DOWN | keys::END => {
                    self.move_selection(ScrollType::End)?
                }
                keys::PAGE_UP => {
                    self.move_selection(ScrollType::PageUp)?
                }
                keys::PAGE_DOWN => {
                    self.move_selection(ScrollType::PageDown)?
                }
                _ => false,
            };

            return Ok(selection_changed);
        }

        Ok(false)
    }

    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        _force_all: bool,
    ) -> CommandBlocking {
        out.push(CommandInfo::new(
            commands::SCROLL,
            self.selected_entry().is_some(),
            true,
        ));
        CommandBlocking::PassingOn
    }
}

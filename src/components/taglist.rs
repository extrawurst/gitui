use super::{
    utils, visibility_blocking, CommandBlocking, CommandInfo,
    Component, DrawableComponent, EventState,
};
use crate::{
    components::ScrollType,
    keys::SharedKeyConfig,
    queue::{Action, InternalEvent, Queue},
    strings,
    ui::{self, Size},
};
use anyhow::Result;
use asyncgit::{
    asyncjob::AsyncSingleJob,
    remote_tags::AsyncRemoteTagsJob,
    sync::cred::{
        extract_username_password, need_username_password,
        BasicAuthCredential,
    },
    sync::{get_tags_with_metadata, TagWithMetadata},
    AsyncGitNotification, CWD,
};
use crossbeam_channel::Sender;
use crossterm::event::Event;
use std::convert::TryInto;
use tui::{
    backend::Backend,
    layout::{Constraint, Margin, Rect},
    text::Span,
    widgets::{
        Block, BorderType, Borders, Cell, Clear, Row, Table,
        TableState,
    },
    Frame,
};
use ui::style::SharedTheme;

///
pub struct TagListComponent {
    theme: SharedTheme,
    queue: Queue,
    tags: Option<Vec<TagWithMetadata>>,
    visible: bool,
    table_state: std::cell::Cell<TableState>,
    current_height: std::cell::Cell<usize>,
    missing_remote_tags: Option<Vec<String>>,
    basic_credential: Option<BasicAuthCredential>,
    async_remote_tags:
        AsyncSingleJob<AsyncRemoteTagsJob, AsyncGitNotification>,
    key_config: SharedKeyConfig,
}

impl DrawableComponent for TagListComponent {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        rect: Rect,
    ) -> Result<()> {
        if self.visible {
            const PERCENT_SIZE: Size = Size::new(80, 50);
            const MIN_SIZE: Size = Size::new(60, 20);

            let area = ui::centered_rect(
                PERCENT_SIZE.width,
                PERCENT_SIZE.height,
                f.size(),
            );
            let area =
                ui::rect_inside(MIN_SIZE, f.size().into(), area);
            let area = area.intersection(rect);

            let tag_name_width =
                self.tags.as_ref().map_or(0, |tags| {
                    tags.iter()
                        .fold(0, |acc, tag| acc.max(tag.name.len()))
                });

            let constraints = [
                // symbol if tag is not yet on remote and can be pushed
                Constraint::Length(1),
                // tag name
                Constraint::Length(tag_name_width.try_into()?),
                // commit date
                Constraint::Length(10),
                // author width
                Constraint::Length(19),
                // commit id
                Constraint::Min(0),
            ];

            let rows = self.get_rows();
            let number_of_rows = rows.len();

            let table = Table::new(rows)
                .widths(&constraints)
                .column_spacing(1)
                .highlight_style(self.theme.text(true, true))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(Span::styled(
                            strings::title_tags(),
                            self.theme.title(true),
                        ))
                        .border_style(self.theme.block(true))
                        .border_type(BorderType::Thick),
                );

            let mut table_state = self.table_state.take();

            f.render_widget(Clear, area);
            f.render_stateful_widget(table, area, &mut table_state);

            let area = area.inner(&Margin {
                vertical: 1,
                horizontal: 0,
            });

            ui::draw_scrollbar(
                f,
                area,
                &self.theme,
                number_of_rows,
                table_state.selected().unwrap_or(0),
            );

            self.table_state.set(table_state);
            self.current_height.set(area.height.into());
        }

        Ok(())
    }
}

impl Component for TagListComponent {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        if self.visible || force_all {
            if !force_all {
                out.clear();
            }

            out.push(CommandInfo::new(
                strings::commands::scroll(&self.key_config),
                true,
                true,
            ));

            out.push(CommandInfo::new(
                strings::commands::close_popup(&self.key_config),
                true,
                true,
            ));

            out.push(CommandInfo::new(
                strings::commands::delete_tag_popup(&self.key_config),
                self.valid_selection(),
                true,
            ));
            out.push(CommandInfo::new(
                strings::commands::select_tag(&self.key_config),
                self.valid_selection(),
                true,
            ));
            out.push(CommandInfo::new(
                strings::commands::push_tags(&self.key_config),
                true,
                true,
            ));
        }
        visibility_blocking(self)
    }

    fn event(&mut self, event: Event) -> Result<EventState> {
        if self.visible {
            if let Event::Key(key) = event {
                if key == self.key_config.exit_popup {
                    self.hide();
                } else if key == self.key_config.move_up {
                    self.move_selection(ScrollType::Up);
                } else if key == self.key_config.move_down {
                    self.move_selection(ScrollType::Down);
                } else if key == self.key_config.shift_up
                    || key == self.key_config.home
                {
                    self.move_selection(ScrollType::Home);
                } else if key == self.key_config.shift_down
                    || key == self.key_config.end
                {
                    self.move_selection(ScrollType::End);
                } else if key == self.key_config.page_down {
                    self.move_selection(ScrollType::PageDown);
                } else if key == self.key_config.page_up {
                    self.move_selection(ScrollType::PageUp);
                } else if key == self.key_config.delete_tag {
                    return self.selected_tag().map_or(
                        Ok(EventState::NotConsumed),
                        |tag| {
                            self.queue.push(
                                InternalEvent::ConfirmAction(
                                    Action::DeleteTag(
                                        tag.name.clone(),
                                    ),
                                ),
                            );
                            Ok(EventState::Consumed)
                        },
                    );
                } else if key == self.key_config.select_tag {
                    return self.selected_tag().map_or(
                        Ok(EventState::NotConsumed),
                        |tag| {
                            self.queue.push(
                                InternalEvent::SelectCommitInRevlog(
                                    tag.commit_id,
                                ),
                            );
                            Ok(EventState::Consumed)
                        },
                    );
                } else if key == self.key_config.push {
                    self.queue.push(InternalEvent::PushTags);
                }
            }

            Ok(EventState::Consumed)
        } else {
            Ok(EventState::NotConsumed)
        }
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

impl TagListComponent {
    pub fn new(
        queue: &Queue,
        sender: &Sender<AsyncGitNotification>,
        theme: SharedTheme,
        key_config: SharedKeyConfig,
    ) -> Self {
        Self {
            theme,
            queue: queue.clone(),
            tags: None,
            visible: false,
            table_state: std::cell::Cell::new(TableState::default()),
            current_height: std::cell::Cell::new(0),
            basic_credential: None,
            missing_remote_tags: None,
            async_remote_tags: AsyncSingleJob::new(
                sender.clone(),
                AsyncGitNotification::RemoteTags,
            ),
            key_config,
        }
    }

    ///
    pub fn open(&mut self) -> Result<()> {
        self.table_state.get_mut().select(Some(0));
        self.show()?;

        let basic_credential = if need_username_password()? {
            let credential = extract_username_password()?;

            if credential.is_complete() {
                Some(credential)
            } else {
                None
            }
        } else {
            None
        };

        self.basic_credential = basic_credential;

        self.update_tags()?;
        self.update_missing_remote_tags();

        Ok(())
    }

    ///
    pub fn update(&mut self, event: AsyncGitNotification) {
        if event == AsyncGitNotification::RemoteTags {
            if let Some(job) = self.async_remote_tags.take_last() {
                if let Some(Ok(missing_remote_tags)) = job.result() {
                    self.missing_remote_tags =
                        Some(missing_remote_tags);
                }
            }
        } else if event == AsyncGitNotification::PushTags {
            self.update_missing_remote_tags();
        }
    }

    ///
    pub fn any_work_pending(&self) -> bool {
        self.async_remote_tags.is_pending()
    }

    /// fetch list of tags
    pub fn update_tags(&mut self) -> Result<()> {
        let tags = get_tags_with_metadata(CWD)?;

        self.tags = Some(tags);

        Ok(())
    }

    pub fn update_missing_remote_tags(&mut self) {
        self.async_remote_tags.spawn(AsyncRemoteTagsJob::new(
            self.basic_credential.clone(),
        ));
    }

    ///
    fn move_selection(&mut self, scroll_type: ScrollType) -> bool {
        let mut table_state = self.table_state.take();

        let old_selection = table_state.selected().unwrap_or(0);
        let max_selection =
            self.tags.as_ref().map_or(0, |tags| tags.len() - 1);

        let new_selection = match scroll_type {
            ScrollType::Up => old_selection.saturating_sub(1),
            ScrollType::Down => {
                old_selection.saturating_add(1).min(max_selection)
            }
            ScrollType::Home => 0,
            ScrollType::End => max_selection,
            ScrollType::PageUp => old_selection.saturating_sub(
                self.current_height.get().saturating_sub(1),
            ),
            ScrollType::PageDown => old_selection
                .saturating_add(
                    self.current_height.get().saturating_sub(1),
                )
                .min(max_selection),
        };

        let needs_update = new_selection != old_selection;

        table_state.select(Some(new_selection));
        self.table_state.set(table_state);

        needs_update
    }

    ///
    fn get_rows(&self) -> Vec<Row> {
        if let Some(ref tags) = self.tags {
            tags.iter().map(|tag| self.get_row(tag)).collect()
        } else {
            vec![]
        }
    }

    ///
    fn get_row(&self, tag: &TagWithMetadata) -> Row {
        const UPSTREAM_SYMBOL: &str = "\u{2191}";
        const EMPTY_SYMBOL: &str = " ";

        let is_tag_missing_on_remote = self
            .missing_remote_tags
            .as_ref()
            .map_or(false, |missing_remote_tags| {
                let remote_tag = format!("refs/tags/{}", tag.name);

                missing_remote_tags.contains(&remote_tag)
            });

        let has_remote_str = if is_tag_missing_on_remote {
            UPSTREAM_SYMBOL
        } else {
            EMPTY_SYMBOL
        };

        let cells: Vec<Cell> = vec![
            Cell::from(has_remote_str)
                .style(self.theme.commit_author(false)),
            Cell::from(tag.name.clone())
                .style(self.theme.text(true, false)),
            Cell::from(utils::time_to_string(tag.time, true))
                .style(self.theme.commit_time(false)),
            Cell::from(tag.author.clone())
                .style(self.theme.commit_author(false)),
            Cell::from(tag.message.clone())
                .style(self.theme.text(true, false)),
        ];

        Row::new(cells)
    }

    fn valid_selection(&self) -> bool {
        self.selected_tag().is_some()
    }

    fn selected_tag(&self) -> Option<&TagWithMetadata> {
        self.tags.as_ref().and_then(|tags| {
            let table_state = self.table_state.take();

            let tag = table_state
                .selected()
                .and_then(|selected| tags.get(selected));

            self.table_state.set(table_state);

            tag
        })
    }
}

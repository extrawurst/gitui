use crate::{
    components::{
        async_commit_filter::{
            AsyncCommitFilterer, FilterBy, FilterStatus,
        },
        visibility_blocking, CommandBlocking, CommandInfo,
        CommitDetailsComponent, CommitList, Component,
        DrawableComponent, FindCommitComponent,
    },
    keys::SharedKeyConfig,
    queue::{InternalEvent, Queue},
    strings,
    ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::{
    cached,
    sync::{self, CommitId},
    AsyncLog, AsyncNotification, AsyncTags, FetchStatus, CWD,
};
use crossbeam_channel::Sender;
use crossterm::event::Event;
use std::time::Duration;
use sync::CommitTags;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

const SLICE_SIZE: usize = 1200;

///
pub struct Revlog {
    commit_details: CommitDetailsComponent,
    list: CommitList,
    find_commit: FindCommitComponent,
    async_filter: AsyncCommitFilterer,
    git_log: AsyncLog,
    git_tags: AsyncTags,
    queue: Queue,
    visible: bool,
    branch_name: cached::BranchName,
    key_config: SharedKeyConfig,
    is_filtering: bool,
}

impl Revlog {
    ///
    pub fn new(
        queue: &Queue,
        sender: &Sender<AsyncNotification>,
        theme: SharedTheme,
        key_config: SharedKeyConfig,
    ) -> Self {
        let log = AsyncLog::new(sender);
        let tags = AsyncTags::new(sender);
        Self {
            queue: queue.clone(),
            commit_details: CommitDetailsComponent::new(
                queue,
                sender,
                theme.clone(),
                key_config.clone(),
            ),
            list: CommitList::new(
                &strings::log_title(&key_config),
                theme.clone(),
                key_config.clone(),
            ),
            find_commit: FindCommitComponent::new(
                queue.clone(),
                theme,
                key_config.clone(),
            ),
            async_filter: AsyncCommitFilterer::new(
                log.clone(),
                tags.clone(),
                sender,
            ),
            git_log: log,
            git_tags: tags,
            visible: false,
            branch_name: cached::BranchName::new(CWD),
            key_config,
            is_filtering: false,
        }
    }

    ///
    pub fn any_work_pending(&self) -> bool {
        self.git_log.is_pending()
            || self.git_tags.is_pending()
            || self.async_filter.is_pending()
            || self.commit_details.any_work_pending()
    }

    ///
    pub fn update(&mut self) -> Result<()> {
        if self.visible {
            let log_changed = if self.is_filtering {
                self.list
                    .update_total_count(self.async_filter.count());
                self.async_filter.fetch() == FilterStatus::Filtering
            } else {
                self.list.update_total_count(self.git_log.count()?);
                self.git_log.fetch()? == FetchStatus::Started
            };

            let selection = self.list.selection();
            let selection_max = self.list.selection_max();
            if self.list.items().needs_data(selection, selection_max)
                || log_changed
            {
                self.fetch_commits()?;
            }

            self.git_tags.request(Duration::from_secs(3), false)?;

            self.list.set_branch(
                self.branch_name.lookup().map(Some).unwrap_or(None),
            );

            if self.commit_details.is_visible() {
                let commit = self.selected_commit();
                let tags = self.selected_commit_tags(&commit);

                self.commit_details.set_commit(commit, tags)?;
            }
        }

        Ok(())
    }

    ///
    pub fn update_git(
        &mut self,
        ev: AsyncNotification,
    ) -> Result<()> {
        if self.visible {
            match ev {
                AsyncNotification::CommitFiles
                | AsyncNotification::Log => self.update()?,
                AsyncNotification::Tags => {
                    if let Some(tags) = self.git_tags.last()? {
                        self.list.set_tags(tags);
                        self.update()?;
                    }
                }
                _ => (),
            }
        }

        Ok(())
    }

    fn fetch_commits(&mut self) -> Result<()> {
        let want_min =
            self.list.selection().saturating_sub(SLICE_SIZE / 2);

        let commits = if self.is_filtering {
            self.async_filter
                .get_filter_items(
                    want_min,
                    SLICE_SIZE,
                    self.list.current_size().0.into(),
                )
                .map_err(|e| anyhow::anyhow!(e.to_string()))
        } else {
            sync::get_commits_info(
                CWD,
                &self.git_log.get_slice(want_min, SLICE_SIZE)?,
                self.list.current_size().0.into(),
            )
            .map_err(|e| anyhow::anyhow!(e.to_string()))
        };

        if let Ok(commits) = commits {
            self.list.items().set_items(want_min, commits);
        };

        Ok(())
    }

    fn selected_commit(&self) -> Option<CommitId> {
        self.list.selected_entry().map(|e| e.id)
    }

    fn copy_commit_hash(&self) -> Result<()> {
        self.list.copy_entry_hash()?;
        Ok(())
    }

    fn selected_commit_tags(
        &self,
        commit: &Option<CommitId>,
    ) -> Option<CommitTags> {
        let tags = self.list.tags();

        commit.and_then(|commit| {
            tags.and_then(|tags| tags.get(&commit).cloned())
        })
    }

    fn get_what_to_filter_by(
        filter_by_str: &str,
    ) -> Vec<Vec<(String, FilterBy)>> {
        let mut search_vec = vec![];
        let mut and_vec = Vec::new();
        for or in filter_by_str.split("||") {
            for split_sub in or.split("&&") {
                if let Some(':') = split_sub.chars().next() {
                    let mut to_filter_by = FilterBy::empty();
                    let mut split_str =
                        split_sub.split(' ').collect::<Vec<&str>>();
                    if split_str.len() == 1 {
                        split_str.push("");
                    }
                    let first = split_str[0];
                    if first.contains('s') {
                        to_filter_by |= FilterBy::SHA;
                    }
                    if first.contains('a') {
                        to_filter_by |= FilterBy::AUTHOR;
                    }
                    if first.contains('m') {
                        to_filter_by |= FilterBy::MESSAGE;
                    }
                    if first.contains('c') {
                        to_filter_by |= FilterBy::CASE_SENSITIVE;
                    }
                    if first.contains('t') {
                        to_filter_by |= FilterBy::TAGS;
                    }
                    if first.contains('!') {
                        to_filter_by |= FilterBy::NOT;
                    }

                    if to_filter_by.is_empty() {
                        to_filter_by = FilterBy::all()
                            & !FilterBy::NOT
                            & !FilterBy::CASE_SENSITIVE;
                    } else if to_filter_by
                        == FilterBy::CASE_SENSITIVE & FilterBy::NOT
                    {
                        FilterBy::all();
                    } else if to_filter_by == FilterBy::NOT {
                        to_filter_by = FilterBy::all()
                            & !FilterBy::CASE_SENSITIVE
                            & !FilterBy::TAGS;
                    } else if to_filter_by == FilterBy::CASE_SENSITIVE
                    {
                        to_filter_by =
                            FilterBy::all() & !FilterBy::NOT;
                    };

                    and_vec.push((
                        split_str[1..].join(" ").trim().to_string(),
                        to_filter_by,
                    ));
                } else {
                    and_vec.push((
                        split_sub.to_string(),
                        FilterBy::all()
                            & !FilterBy::NOT
                            & !FilterBy::CASE_SENSITIVE,
                    ))
                }
            }
            search_vec.push(and_vec.clone());
            and_vec.clear();
        }
        search_vec
    }

    pub fn filter(&mut self, filter_by: &str) -> Result<()> {
        let pre_processed_string =
            Self::pre_process_string(filter_by.to_string());
        println!("{}", pre_processed_string);
        let trimmed_string = pre_processed_string.trim().to_string();
        if filter_by == "" {
            self.async_filter.stop_filter();
            self.is_filtering = false;
        } else {
            let filter_strings =
                Self::get_what_to_filter_by(&trimmed_string);
            self.async_filter
                .start_filter(filter_strings)
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            self.is_filtering = true;
        }
        self.update()
    }

    /// pre process string to remove any brackets
    pub fn pre_process_string(mut s: String) -> String {
        while s.contains("&&(") {
            let before = s.clone();
            s = Self::remove_out_brackets(&s);
            if s == before {
                break;
            }
        }
        s
    }

    pub fn remove_out_brackets(s: &str) -> String {
        if let Some(first_bracket) = s.find("&&(") {
            let (first, rest_of_string) =
                s.split_at(first_bracket + 3);
            if let Some(last_bracket) =
                Self::get_ending_bracket(rest_of_string)
            {
                let mut v = vec![];
                let (second, third) =
                    rest_of_string.split_at(last_bracket);
                if let Some((first, third)) = first
                    .strip_suffix('(')
                    .zip(third.strip_prefix(')'))
                {
                    for element in second.split("||") {
                        // Append first, prepend third onto branket element
                        v.push(format!(
                            "{}{}{}",
                            first, element, third
                        ));
                    }
                    return v.join("||");
                }
            }
        }
        s.to_string()
    }

    pub fn get_ending_bracket(s: &str) -> Option<usize> {
        let mut brack_count = 0;
        let mut char_iter = s.chars();
        let mut ending_brakcet_pos = None;
        let mut iter_count: usize = 0;
        while let Some(c) = char_iter.next() {
            if c == '(' {
                brack_count += 1;
            } else if c == ')' {
                if brack_count == 0 {
                    // Found
                    ending_brakcet_pos = Some(iter_count);
                    break;
                }
                brack_count -= 1;
            }
            iter_count += 1;
        }
        ending_brakcet_pos
    }
}

impl DrawableComponent for Revlog {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        area: Rect,
    ) -> Result<()> {
        if self.commit_details.is_visible() {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    [
                        Constraint::Percentage(60),
                        Constraint::Percentage(40),
                    ]
                    .as_ref(),
                )
                .split(area);

            if self.find_commit.is_visible() {
                let log_find_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(
                        [
                            Constraint::Percentage(90),
                            Constraint::Percentage(20),
                        ]
                        .as_ref(),
                    )
                    .split(chunks[0]);
                self.list.draw(f, log_find_chunks[0])?;
                self.find_commit.draw(f, log_find_chunks[1])?;
                self.commit_details.draw(f, chunks[1])?;
            } else {
                self.list.draw(f, chunks[0])?;
                self.commit_details.draw(f, chunks[1])?;
            }
        } else if self.find_commit.is_visible() {
            let log_find_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Percentage(90),
                        Constraint::Percentage(20),
                    ]
                    .as_ref(),
                )
                .split(area);
            self.list.draw(f, log_find_chunks[0])?;
            self.find_commit.draw(f, log_find_chunks[1])?;
        } else {
            self.list.draw(f, area)?;
        }

        Ok(())
    }
}

impl Component for Revlog {
    fn event(&mut self, ev: Event) -> Result<bool> {
        if self.visible {
            let mut event_used = self.find_commit.event(ev)?;
            if !event_used {
                event_used = self.list.event(ev)?;
            }

            if event_used {
                self.update()?;
                return Ok(true);
            } else if let Event::Key(k) = ev {
                if k == self.key_config.enter {
                    self.commit_details.toggle_visible()?;
                    self.update()?;
                    return Ok(true);
                } else if k == self.key_config.copy {
                    self.copy_commit_hash()?;
                    return Ok(true);
                } else if k == self.key_config.log_tag_commit {
                    return self.selected_commit().map_or(
                        Ok(false),
                        |id| {
                            self.queue.borrow_mut().push_back(
                                InternalEvent::TagCommit(id),
                            );
                            Ok(true)
                        },
                    );
                } else if k == self.key_config.focus_right
                    && self.commit_details.is_visible()
                {
                    return self.selected_commit().map_or(
                        Ok(false),
                        |id| {
                            self.queue.borrow_mut().push_back(
                                InternalEvent::InspectCommit(
                                    id,
                                    self.selected_commit_tags(&Some(
                                        id,
                                    )),
                                ),
                            );
                            Ok(true)
                        },
                    );
                } else if k == self.key_config.select_branch {
                    self.queue
                        .borrow_mut()
                        .push_back(InternalEvent::SelectBranch);
                    return Ok(true);
                } else if k
                    == self.key_config.show_find_commit_text_input
                {
                    self.find_commit.toggle_visible()?;
                    self.find_commit.focus(true);
                    return Ok(true);
                } else if k == self.key_config.focus_find_commit {
                    self.find_commit.focus(true);
                    return Ok(true);
                } else if k == self.key_config.exit_popup {
                    self.filter("")?;
                    self.find_commit.clear_input();
                    self.update()?;
                }
            }
        }

        Ok(false)
    }

    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        if self.visible || force_all {
            self.list.commands(out, force_all);
        }

        out.push(CommandInfo::new(
            strings::commands::log_details_toggle(&self.key_config),
            true,
            self.visible,
        ));

        out.push(CommandInfo::new(
            strings::commands::log_details_open(&self.key_config),
            true,
            (self.visible && self.commit_details.is_visible())
                || force_all,
        ));

        out.push(CommandInfo::new(
            strings::commands::log_tag_commit(&self.key_config),
            true,
            self.visible || force_all,
        ));

        out.push(CommandInfo::new(
            strings::commands::open_branch_select_popup(
                &self.key_config,
            ),
            true,
            self.visible || force_all,
        ));

        out.push(CommandInfo::new(
            strings::commands::copy_hash(&self.key_config),
            true,
            self.visible || force_all,
        ));

        out.push(CommandInfo::new(
            strings::commands::find_commit(&self.key_config),
            true,
            self.visible || force_all,
        ));

        visibility_blocking(self)
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn hide(&mut self) {
        self.visible = false;
        self.git_log.set_background();
    }

    fn show(&mut self) -> Result<()> {
        self.visible = true;
        self.list.clear();
        self.update()?;

        Ok(())
    }
}

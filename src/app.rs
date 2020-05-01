use crate::{
    components::{
        ChangesComponent, CommandBlocking, CommandInfo,
        CommitComponent, Component, DiffComponent, DrawableComponent,
        FileTreeItemKind, HelpComponent, MsgComponent,
        ResetComponent,
    },
    keys,
    queue::{InternalEvent, NeedsUpdate, Queue},
    strings,
};
use asyncgit::{
    current_tick, sync, AsyncDiff, AsyncNotification, AsyncStatus,
    DiffParams, CWD,
};
use crossbeam_channel::Sender;
use crossterm::event::Event;
use itertools::Itertools;
use log::{debug, trace};
use std::borrow::Cow;
use strings::commands;
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Tabs, Text},
    Frame,
};

///
#[derive(PartialEq)]
enum DiffTarget {
    Stage,
    WorkingDir,
}

///
#[derive(PartialEq)]
enum Focus {
    WorkDir,
    Diff,
    Stage,
}

/// allows generating code to make sure
/// we always enumerate all components in both getter functions
macro_rules! components {
    ($self:ident, [$($element:ident),+]) => {
        fn components(& $self) -> Vec<&dyn Component> {
            vec![
                $(&$self.$element,)+
            ]
        }

        fn components_mut(&mut $self) -> Vec<&mut dyn Component> {
            vec![
                $(&mut $self.$element,)+
            ]
        }
    };
}

///
pub struct App {
    focus: Focus,
    diff_target: DiffTarget,
    do_quit: bool,
    reset: ResetComponent,
    commit: CommitComponent,
    help: HelpComponent,
    index: ChangesComponent,
    index_wd: ChangesComponent,
    diff: DiffComponent,
    msg: MsgComponent,
    git_diff: AsyncDiff,
    git_status: AsyncStatus,
    current_commands: Vec<CommandInfo>,
    queue: Queue,
}

// public interface
impl App {
    ///
    pub fn new(sender: Sender<AsyncNotification>) -> Self {
        let queue = Queue::default();
        Self {
            focus: Focus::WorkDir,
            diff_target: DiffTarget::WorkingDir,
            do_quit: false,
            reset: ResetComponent::new(queue.clone()),
            commit: CommitComponent::new(queue.clone()),
            help: HelpComponent::default(),
            index_wd: ChangesComponent::new(
                strings::TITLE_STATUS,
                true,
                true,
                queue.clone(),
            ),
            index: ChangesComponent::new(
                strings::TITLE_INDEX,
                false,
                false,
                queue.clone(),
            ),
            diff: DiffComponent::new(queue.clone()),
            msg: MsgComponent::default(),
            git_diff: AsyncDiff::new(sender.clone()),
            git_status: AsyncStatus::new(sender),
            current_commands: Vec::new(),
            queue,
        }
    }

    ///
    pub fn draw<B: Backend>(&self, f: &mut Frame<B>) {
        let chunks_main = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(2),
                    Constraint::Min(2),
                    Constraint::Length(1),
                ]
                .as_ref(),
            )
            .split(f.size());

        f.render_widget(
            Tabs::default()
                .block(Block::default().borders(Borders::BOTTOM))
                .titles(&[strings::TAB_STATUS])
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().fg(Color::Yellow))
                .divider(strings::TAB_DIVIDER),
            chunks_main[0],
        );

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                if self.focus == Focus::Diff {
                    [
                        Constraint::Percentage(30),
                        Constraint::Percentage(70),
                    ]
                } else {
                    [
                        Constraint::Percentage(50),
                        Constraint::Percentage(50),
                    ]
                }
                .as_ref(),
            )
            .split(chunks_main[1]);

        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                if self.diff_target == DiffTarget::WorkingDir {
                    [
                        Constraint::Percentage(60),
                        Constraint::Percentage(40),
                    ]
                } else {
                    [
                        Constraint::Percentage(40),
                        Constraint::Percentage(60),
                    ]
                }
                .as_ref(),
            )
            .split(chunks[0]);

        self.index_wd.draw(f, left_chunks[0]);
        self.index.draw(f, left_chunks[1]);
        self.diff.draw(f, chunks[1]);

        Self::draw_commands(
            f,
            chunks_main[2],
            self.current_commands.as_slice(),
        );

        self.draw_popups(f);
    }

    ///
    pub fn event(&mut self, ev: Event) {
        trace!("event: {:?}", ev);

        let mut flags = NeedsUpdate::empty();

        if Self::event_pump(ev, self.components_mut().as_mut_slice())
        {
            flags.insert(NeedsUpdate::COMMANDS);
        } else if let Event::Key(k) = ev {
            let new_flags = match k {
                keys::EXIT_1 | keys::EXIT_2 => {
                    self.do_quit = true;
                    NeedsUpdate::empty()
                }
                keys::FOCUS_WORKDIR => {
                    self.switch_focus(Focus::WorkDir)
                }
                keys::FOCUS_STAGE => self.switch_focus(Focus::Stage),
                keys::FOCUS_RIGHT if self.can_focus_diff() => {
                    self.switch_focus(Focus::Diff)
                }
                keys::FOCUS_LEFT => {
                    self.switch_focus(match self.diff_target {
                        DiffTarget::Stage => Focus::Stage,
                        DiffTarget::WorkingDir => Focus::WorkDir,
                    })
                }
                keys::OPEN_COMMIT
                    if !self.index.is_empty()
                        && self.offer_open_commit_cmd() =>
                {
                    self.commit.show();
                    NeedsUpdate::COMMANDS
                }
                keys::LOG_TEST => {
                    let log_len = sync::get_log_len(CWD).unwrap();
                    debug!("log_len: {}", log_len);
                    NeedsUpdate::empty()
                }
                _ => NeedsUpdate::empty(),
            };

            flags.insert(new_flags);
        }

        let new_flags = self.process_queue();
        flags.insert(new_flags);

        if flags.contains(NeedsUpdate::ALL) {
            self.update();
        }
        if flags.contains(NeedsUpdate::DIFF) {
            self.update_diff();
        }
        if flags.contains(NeedsUpdate::COMMANDS) {
            self.update_commands();
        }
    }

    ///
    pub fn update(&mut self) {
        trace!("update");

        self.git_diff.refresh();
        self.git_status.fetch(current_tick());
    }

    ///
    pub fn update_git(&mut self, ev: AsyncNotification) {
        trace!("update_git: {:?}", ev);
        match ev {
            AsyncNotification::Diff => self.update_diff(),
            AsyncNotification::Status => self.update_status(),
        }
    }

    ///
    pub fn is_quit(&self) -> bool {
        self.do_quit
    }

    ///
    pub fn any_work_pending(&self) -> bool {
        self.git_diff.is_pending() || self.git_status.is_pending()
    }
}

// private impls
impl App {
    components!(
        self,
        [msg, reset, commit, help, index, index_wd, diff]
    );

    fn update_diff(&mut self) {
        if let Some((path, is_stage)) = self.selected_path() {
            let diff_params = DiffParams(path.clone(), is_stage);

            if self.diff.current() == (path.clone(), is_stage) {
                // we are already showing a diff of the right file
                // maybe the diff changed (outside file change)
                if let Some(last) = self.git_diff.last() {
                    self.diff.update(path, is_stage, last);
                }
            } else {
                // we dont show the right diff right now, so we need to request
                if let Some(diff) = self.git_diff.request(diff_params)
                {
                    self.diff.update(path, is_stage, diff);
                } else {
                    self.diff.clear();
                }
            }
        } else {
            self.diff.clear();
        }
    }

    fn selected_path(&self) -> Option<(String, bool)> {
        let (idx, is_stage) = match self.diff_target {
            DiffTarget::Stage => (&self.index, true),
            DiffTarget::WorkingDir => (&self.index_wd, false),
        };

        if let Some(item) = idx.selection() {
            if let FileTreeItemKind::File(i) = item.kind {
                return Some((i.path, is_stage));
            }
        }
        None
    }

    fn can_focus_diff(&self) -> bool {
        match self.focus {
            Focus::WorkDir => self.index_wd.is_file_seleted(),
            Focus::Stage => self.index.is_file_seleted(),
            _ => false,
        }
    }

    fn update_commands(&mut self) {
        self.help.set_cmds(self.commands(true));
        self.current_commands = self.commands(false);
        self.current_commands.sort_by_key(|e| e.order);
    }

    fn update_status(&mut self) {
        let status = self.git_status.last();
        self.index.update(&status.stage);
        self.index_wd.update(&status.work_dir);

        self.update_diff();
        self.update_commands();
    }

    fn process_queue(&mut self) -> NeedsUpdate {
        let mut flags = NeedsUpdate::empty();
        loop {
            let front = self.queue.borrow_mut().pop_front();
            if let Some(e) = front {
                flags.insert(self.process_internal_event(e));
            } else {
                break;
            }
        }
        self.queue.borrow_mut().clear();

        flags
    }

    fn process_internal_event(
        &mut self,
        ev: InternalEvent,
    ) -> NeedsUpdate {
        let mut flags = NeedsUpdate::empty();
        match ev {
            InternalEvent::ResetItem(reset_item) => {
                if reset_item.is_folder {
                    if sync::reset_workdir_folder(
                        CWD,
                        reset_item.path.as_str(),
                    ) {
                        flags.insert(NeedsUpdate::ALL);
                    }
                } else if sync::reset_workdir_file(
                    CWD,
                    reset_item.path.as_str(),
                ) {
                    flags.insert(NeedsUpdate::ALL);
                }
            }
            InternalEvent::ConfirmResetItem(reset_item) => {
                self.reset.open_for_path(reset_item);
                flags.insert(NeedsUpdate::COMMANDS);
            }
            InternalEvent::AddHunk(hash) => {
                if let Some((path, is_stage)) = self.selected_path() {
                    if is_stage {
                        if sync::unstage_hunk(CWD, path, hash) {
                            flags.insert(NeedsUpdate::ALL);
                        }
                    } else if sync::stage_hunk(CWD, path, hash) {
                        flags.insert(NeedsUpdate::ALL);
                    }
                }
            }
            InternalEvent::ShowMsg(msg) => {
                self.msg.show_msg(msg.as_str());
                flags.insert(NeedsUpdate::ALL);
            }
            InternalEvent::Update(u) => flags.insert(u),
        };

        flags
    }

    fn commands(&self, force_all: bool) -> Vec<CommandInfo> {
        let mut res = Vec::new();

        for c in self.components() {
            if c.commands(&mut res, force_all)
                != CommandBlocking::PassingOn
                && !force_all
            {
                break;
            }
        }

        let main_cmds_available = !self.any_popup_visible();

        {
            {
                let focus_on_stage = self.focus == Focus::Stage;
                let focus_not_diff = self.focus != Focus::Diff;
                res.push(
                    CommandInfo::new(
                        commands::STATUS_FOCUS_UNSTAGED,
                        true,
                        main_cmds_available
                            && focus_on_stage
                            && !focus_not_diff,
                    )
                    .hidden(),
                );
                res.push(
                    CommandInfo::new(
                        commands::STATUS_FOCUS_STAGED,
                        true,
                        main_cmds_available
                            && !focus_on_stage
                            && !focus_not_diff,
                    )
                    .hidden(),
                );
            }
            {
                let focus_on_diff = self.focus == Focus::Diff;
                res.push(CommandInfo::new(
                    commands::STATUS_FOCUS_LEFT,
                    true,
                    main_cmds_available && focus_on_diff,
                ));
                res.push(CommandInfo::new(
                    commands::STATUS_FOCUS_RIGHT,
                    self.can_focus_diff(),
                    main_cmds_available && !focus_on_diff,
                ));
            }

            res.push(
                CommandInfo::new(
                    commands::COMMIT_OPEN,
                    !self.index.is_empty(),
                    self.offer_open_commit_cmd(),
                )
                .order(-1),
            );

            res.push(
                CommandInfo::new(
                    commands::SELECT_STAGING,
                    true,
                    self.focus == Focus::WorkDir,
                )
                .order(-2),
            );

            res.push(
                CommandInfo::new(
                    commands::SELECT_UNSTAGED,
                    true,
                    self.focus == Focus::Stage,
                )
                .order(-2),
            );

            res.push(
                CommandInfo::new(
                    commands::QUIT,
                    true,
                    main_cmds_available,
                )
                .order(100),
            );
        }

        res
    }

    fn offer_open_commit_cmd(&self) -> bool {
        !self.commit.is_visible()
            && self.diff_target == DiffTarget::Stage
    }

    fn event_pump(
        ev: Event,
        components: &mut [&mut dyn Component],
    ) -> bool {
        for c in components {
            if c.event(ev) {
                return true;
            }
        }

        false
    }

    fn any_popup_visible(&self) -> bool {
        self.commit.is_visible()
            || self.help.is_visible()
            || self.reset.is_visible()
            || self.msg.is_visible()
    }

    fn draw_popups<B: Backend>(&self, f: &mut Frame<B>) {
        let size = f.size();

        self.commit.draw(f, size);
        self.reset.draw(f, size);
        self.help.draw(f, size);
        self.msg.draw(f, size);
    }

    fn draw_commands<B: Backend>(
        f: &mut Frame<B>,
        r: Rect,
        cmds: &[CommandInfo],
    ) {
        let splitter = Text::Styled(
            Cow::from(strings::CMD_SPLITTER),
            Style::default().bg(Color::Black),
        );

        let style_enabled =
            Style::default().fg(Color::White).bg(Color::Blue);

        let style_disabled =
            Style::default().fg(Color::DarkGray).bg(Color::Blue);
        let texts = cmds
            .iter()
            .filter_map(|c| {
                if c.show_in_quickbar() {
                    Some(Text::Styled(
                        Cow::from(c.text.name),
                        if c.enabled {
                            style_enabled
                        } else {
                            style_disabled
                        },
                    ))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        f.render_widget(
            Paragraph::new(texts.iter().intersperse(&splitter))
                .alignment(Alignment::Left),
            r,
        );
    }

    fn switch_focus(&mut self, f: Focus) -> NeedsUpdate {
        if self.focus == f {
            NeedsUpdate::empty()
        } else {
            self.focus = f;

            match self.focus {
                Focus::WorkDir => {
                    self.set_diff_target(DiffTarget::WorkingDir);
                    self.diff.focus(false);
                }
                Focus::Stage => {
                    self.set_diff_target(DiffTarget::Stage);
                    self.diff.focus(false);
                }
                Focus::Diff => {
                    self.index.focus(false);
                    self.index_wd.focus(false);

                    self.diff.focus(true);
                }
            };

            NeedsUpdate::DIFF | NeedsUpdate::COMMANDS
        }
    }

    fn set_diff_target(&mut self, target: DiffTarget) {
        self.diff_target = target;
        let is_stage = self.diff_target == DiffTarget::Stage;

        self.index_wd.focus_select(!is_stage);
        self.index.focus_select(is_stage);
    }
}

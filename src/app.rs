use crate::{
    accessors,
    cmdbar::CommandBar,
    components::{
        event_pump, CommandBlocking, CommandInfo, CommitComponent,
        Component, CreateBranchComponent, DrawableComponent,
        ExternalEditorComponent, HelpComponent,
        InspectCommitComponent, MsgComponent, PushComponent,
        RenameBranchComponent, ResetComponent, SelectBranchComponent,
        StashMsgComponent, TagCommitComponent,
    },
    input::{Input, InputEvent, InputState},
    keys::{KeyConfig, SharedKeyConfig},
    queue::{Action, InternalEvent, NeedsUpdate, Queue},
    strings::{self, order},
    tabs::{Revlog, StashList, Stashing, Status},
    ui::style::{SharedTheme, Theme},
};
use anyhow::{bail, Result};
use asyncgit::{sync, AsyncNotification, CWD};
use crossbeam_channel::Sender;
use crossterm::event::{Event, KeyEvent};
use std::{
    cell::{Cell, RefCell},
    path::{Path, PathBuf},
    rc::Rc,
};
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    text::{Span, Spans},
    widgets::{Block, Borders, Tabs},
    Frame,
};

///
pub struct App {
    do_quit: bool,
    help: HelpComponent,
    msg: MsgComponent,
    reset: ResetComponent,
    commit: CommitComponent,
    stashmsg_popup: StashMsgComponent,
    inspect_commit_popup: InspectCommitComponent,
    external_editor_popup: ExternalEditorComponent,
    push_popup: PushComponent,
    tag_commit_popup: TagCommitComponent,
    create_branch_popup: CreateBranchComponent,
    rename_branch_popup: RenameBranchComponent,
    select_branch_popup: SelectBranchComponent,
    cmdbar: RefCell<CommandBar>,
    tab: usize,
    revlog: Revlog,
    status_tab: Status,
    stashing_tab: Stashing,
    stashlist_tab: StashList,
    queue: Queue,
    theme: SharedTheme,
    key_config: SharedKeyConfig,
    input: Input,

    // "Flags"
    requires_redraw: Cell<bool>,
    file_to_open: Option<String>,
}

// public interface
impl App {
    ///
    pub fn new(
        sender: &Sender<AsyncNotification>,
        input: Input,
        theme_path: PathBuf,
    ) -> Self {
        let queue = Queue::default();

        let theme = Rc::new(Theme::init(theme_path));
        let key_config = Rc::new(KeyConfig::init());

        Self {
            input,
            reset: ResetComponent::new(
                queue.clone(),
                theme.clone(),
                key_config.clone(),
            ),
            commit: CommitComponent::new(
                queue.clone(),
                theme.clone(),
                key_config.clone(),
            ),
            stashmsg_popup: StashMsgComponent::new(
                queue.clone(),
                theme.clone(),
                key_config.clone(),
            ),
            inspect_commit_popup: InspectCommitComponent::new(
                &queue,
                sender,
                theme.clone(),
                key_config.clone(),
            ),
            external_editor_popup: ExternalEditorComponent::new(
                theme.clone(),
                key_config.clone(),
            ),
            push_popup: PushComponent::new(
                &queue,
                sender,
                theme.clone(),
                key_config.clone(),
            ),
            tag_commit_popup: TagCommitComponent::new(
                queue.clone(),
                theme.clone(),
                key_config.clone(),
            ),
            create_branch_popup: CreateBranchComponent::new(
                queue.clone(),
                theme.clone(),
                key_config.clone(),
            ),
            rename_branch_popup: RenameBranchComponent::new(
                queue.clone(),
                theme.clone(),
                key_config.clone(),
            ),
            select_branch_popup: SelectBranchComponent::new(
                queue.clone(),
                theme.clone(),
                key_config.clone(),
            ),
            do_quit: false,
            cmdbar: RefCell::new(CommandBar::new(
                theme.clone(),
                key_config.clone(),
            )),
            help: HelpComponent::new(
                theme.clone(),
                key_config.clone(),
            ),
            msg: MsgComponent::new(theme.clone(), key_config.clone()),
            tab: 0,
            revlog: Revlog::new(
                &queue,
                sender,
                theme.clone(),
                key_config.clone(),
            ),
            status_tab: Status::new(
                &queue,
                sender,
                theme.clone(),
                key_config.clone(),
            ),
            stashing_tab: Stashing::new(
                sender,
                &queue,
                theme.clone(),
                key_config.clone(),
            ),
            stashlist_tab: StashList::new(
                &queue,
                theme.clone(),
                key_config.clone(),
            ),
            queue,
            theme,
            key_config,
            requires_redraw: Cell::new(false),
            file_to_open: None,
        }
    }

    ///
    pub fn draw<B: Backend>(&self, f: &mut Frame<B>) -> Result<()> {
        let fsize = f.size();

        self.cmdbar.borrow_mut().refresh_width(fsize.width);

        let chunks_main = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(2),
                    Constraint::Min(2),
                    Constraint::Length(self.cmdbar.borrow().height()),
                ]
                .as_ref(),
            )
            .split(fsize);

        self.cmdbar.borrow().draw(f, chunks_main[2]);

        self.draw_tabs(f, chunks_main[0]);

        //TODO: macro because of generic draw call
        match self.tab {
            0 => self.status_tab.draw(f, chunks_main[1])?,
            1 => self.revlog.draw(f, chunks_main[1])?,
            2 => self.stashing_tab.draw(f, chunks_main[1])?,
            3 => self.stashlist_tab.draw(f, chunks_main[1])?,
            _ => bail!("unknown tab"),
        };

        self.draw_popups(f)?;

        Ok(())
    }

    ///
    pub fn event(&mut self, ev: InputEvent) -> Result<()> {
        log::trace!("event: {:?}", ev);

        if let InputEvent::Input(ev) = ev {
            if self.check_quit_key(ev) {
                return Ok(());
            }

            let mut flags = NeedsUpdate::empty();

            if event_pump(ev, self.components_mut().as_mut_slice())? {
                flags.insert(NeedsUpdate::COMMANDS);
            } else if let Event::Key(k) = ev {
                let new_flags = if k == self.key_config.tab_toggle {
                    self.toggle_tabs(false)?;
                    NeedsUpdate::COMMANDS
                } else if k == self.key_config.tab_toggle_reverse {
                    self.toggle_tabs(true)?;
                    NeedsUpdate::COMMANDS
                } else if k == self.key_config.tab_status
                    || k == self.key_config.tab_log
                    || k == self.key_config.tab_stashing
                    || k == self.key_config.tab_stashes
                {
                    self.switch_tab(k)?;
                    NeedsUpdate::COMMANDS
                } else if k == self.key_config.cmd_bar_toggle {
                    self.cmdbar.borrow_mut().toggle_more();
                    NeedsUpdate::empty()
                } else {
                    NeedsUpdate::empty()
                };

                flags.insert(new_flags);
            }

            self.process_queue(flags)?;
        } else if let InputEvent::State(polling_state) = ev {
            self.external_editor_popup.hide();
            if let InputState::Paused = polling_state {
                let result = match self.file_to_open.take() {
                    Some(path) => {
                        ExternalEditorComponent::open_file_in_editor(
                            Path::new(&path),
                        )
                    }
                    None => self.commit.show_editor(),
                };

                if let Err(e) = result {
                    let msg =
                        format!("failed to launch editor:\n{}", e);
                    log::error!("{}", msg.as_str());
                    self.msg.show_error(msg.as_str())?;
                }

                self.requires_redraw.set(true);
                self.input.set_polling(true);
            }
        }

        Ok(())
    }

    //TODO: do we need this?
    /// forward ticking to components that require it
    pub fn update(&mut self) -> Result<()> {
        log::trace!("update");

        self.status_tab.update()?;
        self.revlog.update()?;
        self.stashing_tab.update()?;
        self.stashlist_tab.update()?;

        self.update_commands();

        Ok(())
    }

    ///
    pub fn update_git(
        &mut self,
        ev: AsyncNotification,
    ) -> Result<()> {
        log::trace!("update_git: {:?}", ev);

        self.status_tab.update_git(ev)?;
        self.stashing_tab.update_git(ev)?;
        self.revlog.update_git(ev)?;
        self.inspect_commit_popup.update_git(ev)?;
        self.push_popup.update_git(ev)?;

        //TODO: better system for this
        // can we simply process the queue here and everyone just uses the queue to schedule a cmd update?
        self.process_queue(NeedsUpdate::COMMANDS)?;

        Ok(())
    }

    ///
    pub const fn is_quit(&self) -> bool {
        self.do_quit
    }

    ///
    pub fn any_work_pending(&self) -> bool {
        self.status_tab.anything_pending()
            || self.revlog.any_work_pending()
            || self.stashing_tab.anything_pending()
            || self.inspect_commit_popup.any_work_pending()
            || self.input.is_state_changing()
    }

    ///
    pub fn requires_redraw(&self) -> bool {
        if self.requires_redraw.get() {
            self.requires_redraw.set(false);
            true
        } else {
            false
        }
    }
}

// private impls
impl App {
    accessors!(
        self,
        [
            msg,
            reset,
            commit,
            stashmsg_popup,
            inspect_commit_popup,
            external_editor_popup,
            push_popup,
            tag_commit_popup,
            create_branch_popup,
            rename_branch_popup,
            select_branch_popup,
            revlog,
            status_tab,
            stashing_tab,
            stashlist_tab,
            help
        ]
    );

    fn check_quit_key(&mut self, ev: Event) -> bool {
        if let Event::Key(e) = ev {
            if e == self.key_config.exit {
                self.do_quit = true;
                return true;
            }
        }
        false
    }

    fn get_tabs(&mut self) -> Vec<&mut dyn Component> {
        vec![
            &mut self.status_tab,
            &mut self.revlog,
            &mut self.stashing_tab,
            &mut self.stashlist_tab,
        ]
    }

    fn toggle_tabs(&mut self, reverse: bool) -> Result<()> {
        let tabs_len = self.get_tabs().len();
        let new_tab = if reverse {
            self.tab.wrapping_sub(1).min(tabs_len.saturating_sub(1))
        } else {
            self.tab.saturating_add(1) % tabs_len
        };

        self.set_tab(new_tab)
    }

    fn switch_tab(&mut self, k: KeyEvent) -> Result<()> {
        if k == self.key_config.tab_status {
            self.set_tab(0)?
        } else if k == self.key_config.tab_log {
            self.set_tab(1)?
        } else if k == self.key_config.tab_stashing {
            self.set_tab(2)?
        } else if k == self.key_config.tab_stashes {
            self.set_tab(3)?
        }

        Ok(())
    }

    fn set_tab(&mut self, tab: usize) -> Result<()> {
        let tabs = self.get_tabs();
        for (i, t) in tabs.into_iter().enumerate() {
            if tab == i {
                t.show()?;
            } else {
                t.hide();
            }
        }

        self.tab = tab;

        Ok(())
    }

    fn update_commands(&mut self) {
        self.help.set_cmds(self.commands(true));
        self.cmdbar.borrow_mut().set_cmds(self.commands(false));
    }

    fn process_queue(&mut self, flags: NeedsUpdate) -> Result<()> {
        let mut flags = flags;
        let new_flags = self.process_internal_events()?;
        flags.insert(new_flags);

        if flags.contains(NeedsUpdate::ALL) {
            self.update()?;
        }
        //TODO: make this a queue event?
        //NOTE: set when any tree component changed selection
        if flags.contains(NeedsUpdate::DIFF) {
            self.status_tab.update_diff()?;
            self.inspect_commit_popup.update_diff()?;
        }
        if flags.contains(NeedsUpdate::COMMANDS) {
            self.update_commands();
        }

        Ok(())
    }

    fn process_internal_events(&mut self) -> Result<NeedsUpdate> {
        let mut flags = NeedsUpdate::empty();

        loop {
            let front = self.queue.borrow_mut().pop_front();
            if let Some(e) = front {
                flags.insert(self.process_internal_event(e)?);
            } else {
                break;
            }
        }
        self.queue.borrow_mut().clear();

        Ok(flags)
    }

    fn process_internal_event(
        &mut self,
        ev: InternalEvent,
    ) -> Result<NeedsUpdate> {
        let mut flags = NeedsUpdate::empty();
        match ev {
            InternalEvent::ConfirmedAction(action) => match action {
                Action::Reset(r) => {
                    if self.status_tab.reset(&r) {
                        flags.insert(NeedsUpdate::ALL);
                    }
                }
                Action::StashDrop(s) => {
                    if StashList::drop(s) {
                        flags.insert(NeedsUpdate::ALL);
                    }
                }
                Action::ResetHunk(path, hash) => {
                    sync::reset_hunk(CWD, path, hash)?;
                    flags.insert(NeedsUpdate::ALL);
                }
                Action::DeleteBranch(branch_ref) => {
                    if let Err(e) =
                        sync::delete_branch(CWD, &branch_ref)
                    {
                        self.queue.borrow_mut().push_back(
                            InternalEvent::ShowErrorMsg(
                                e.to_string(),
                            ),
                        )
                    } else {
                        flags.insert(NeedsUpdate::ALL);
                        self.select_branch_popup.hide();
                    }
                }
            },
            InternalEvent::ConfirmAction(action) => {
                self.reset.open(action)?;
                flags.insert(NeedsUpdate::COMMANDS);
            }
            InternalEvent::ShowErrorMsg(msg) => {
                self.msg.show_error(msg.as_str())?;
                flags
                    .insert(NeedsUpdate::ALL | NeedsUpdate::COMMANDS);
            }
            InternalEvent::Update(u) => flags.insert(u),
            InternalEvent::OpenCommit => self.commit.show()?,
            InternalEvent::PopupStashing(opts) => {
                self.stashmsg_popup.options(opts);
                self.stashmsg_popup.show()?
            }
            InternalEvent::TagCommit(id) => {
                self.tag_commit_popup.open(id)?;
            }
            InternalEvent::CreateBranch => {
                self.create_branch_popup.open()?;
            }
            InternalEvent::RenameBranch(branch_ref, cur_name) => {
                self.rename_branch_popup
                    .open(branch_ref, cur_name)?;
            }
            InternalEvent::SelectBranch => {
                self.select_branch_popup.open()?;
            }
            InternalEvent::TabSwitch => self.set_tab(0)?,
            InternalEvent::InspectCommit(id, tags) => {
                self.inspect_commit_popup.open(id, tags)?;
                flags.insert(NeedsUpdate::ALL | NeedsUpdate::COMMANDS)
            }
            InternalEvent::OpenExternalEditor(path) => {
                self.input.set_polling(false);
                self.external_editor_popup.show()?;
                self.file_to_open = path;
                flags.insert(NeedsUpdate::COMMANDS)
            }
            InternalEvent::Push(branch) => {
                self.push_popup.push(branch)?;
                flags.insert(NeedsUpdate::ALL)
            }
            InternalEvent::FilterLog(string_to_fliter_by) => {
                self.revlog.filter(string_to_fliter_by)
            }
        };

        Ok(flags)
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

        res.push(
            CommandInfo::new(
                strings::commands::toggle_tabs(&self.key_config),
                true,
                !self.any_popup_visible(),
            )
            .order(order::NAV),
        );
        res.push(
            CommandInfo::new(
                strings::commands::toggle_tabs_direct(
                    &self.key_config,
                ),
                true,
                !self.any_popup_visible(),
            )
            .order(order::NAV),
        );

        res.push(
            CommandInfo::new(
                strings::commands::quit(&self.key_config),
                true,
                !self.any_popup_visible(),
            )
            .order(100),
        );

        res
    }

    //TODO: make this automatic, i keep forgetting to add popups here
    fn any_popup_visible(&self) -> bool {
        self.commit.is_visible()
            || self.help.is_visible()
            || self.reset.is_visible()
            || self.msg.is_visible()
            || self.stashmsg_popup.is_visible()
            || self.inspect_commit_popup.is_visible()
            || self.external_editor_popup.is_visible()
            || self.tag_commit_popup.is_visible()
            || self.create_branch_popup.is_visible()
            || self.push_popup.is_visible()
            || self.select_branch_popup.is_visible()
            || self.rename_branch_popup.is_visible()
    }

    fn draw_popups<B: Backend>(
        &self,
        f: &mut Frame<B>,
    ) -> Result<()> {
        let size = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Min(1),
                    Constraint::Length(self.cmdbar.borrow().height()),
                ]
                .as_ref(),
            )
            .split(f.size())[0];

        self.commit.draw(f, size)?;
        self.stashmsg_popup.draw(f, size)?;
        self.help.draw(f, size)?;
        self.inspect_commit_popup.draw(f, size)?;
        self.external_editor_popup.draw(f, size)?;
        self.tag_commit_popup.draw(f, size)?;
        self.select_branch_popup.draw(f, size)?;
        self.create_branch_popup.draw(f, size)?;
        self.rename_branch_popup.draw(f, size)?;
        self.push_popup.draw(f, size)?;
        self.reset.draw(f, size)?;
        self.msg.draw(f, size)?;

        Ok(())
    }

    //TODO: make this dynamic
    fn draw_tabs<B: Backend>(&self, f: &mut Frame<B>, r: Rect) {
        let r = r.inner(&Margin {
            vertical: 0,
            horizontal: 1,
        });

        let tabs = [
            Span::raw(strings::tab_status(&self.key_config)),
            Span::raw(strings::tab_log(&self.key_config)),
            Span::raw(strings::tab_stashing(&self.key_config)),
            Span::raw(strings::tab_stashes(&self.key_config)),
        ]
        .iter()
        .cloned()
        .map(Spans::from)
        .collect();

        f.render_widget(
            Tabs::new(tabs)
                .block(
                    Block::default()
                        .borders(Borders::BOTTOM)
                        .border_style(self.theme.block(false)),
                )
                .style(self.theme.tab(false))
                .highlight_style(self.theme.tab(true))
                .divider(strings::tab_divider(&self.key_config))
                .select(self.tab),
            r,
        );
    }
}

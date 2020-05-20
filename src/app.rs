use crate::{
    accessors,
    components::{
        event_pump, CommandBlocking, CommandInfo, CommitComponent,
        Component, DrawableComponent, HelpComponent, MsgComponent,
        ResetComponent,
    },
    keys,
    queue::{InternalEvent, NeedsUpdate, Queue},
    strings,
    tabs::{Revlog, Status},
    ui::style::Theme,
};
use asyncgit::{sync, AsyncNotification, CWD};
use crossbeam_channel::Sender;
use crossterm::event::Event;
use itertools::Itertools;
use log::trace;
use std::borrow::Cow;
use strings::commands;
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Modifier,
    style::Style,
    widgets::{Block, Borders, Paragraph, Tabs, Text},
    Frame,
};

///
pub struct App {
    do_quit: bool,
    help: HelpComponent,
    msg: MsgComponent,
    reset: ResetComponent,
    commit: CommitComponent,
    current_commands: Vec<CommandInfo>,
    tab: usize,
    revlog: Revlog,
    status_tab: Status,
    queue: Queue,
    theme: Theme,
}

// public interface
impl App {
    ///
    pub fn new(sender: &Sender<AsyncNotification>) -> Self {
        let queue = Queue::default();

        let theme = Theme::init();

        Self {
            reset: ResetComponent::new(queue.clone(), &theme),
            commit: CommitComponent::new(queue.clone(), &theme),
            do_quit: false,
            current_commands: Vec::new(),
            help: HelpComponent::new(&theme),
            msg: MsgComponent::default(),
            tab: 0,
            revlog: Revlog::new(&sender, &theme),
            status_tab: Status::new(&sender, &queue, &theme),
            queue,
            theme,
        }
    }

    ///
    pub fn draw<B: Backend>(&mut self, f: &mut Frame<B>) {
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

        self.draw_tabs(f, chunks_main[0]);

        if self.tab == 0 {
            self.status_tab.draw(f, chunks_main[1]);
        } else {
            self.revlog.draw(f, chunks_main[1]);
        }

        Self::draw_commands(
            f,
            chunks_main[2],
            self.current_commands.as_slice(),
            self.theme,
        );

        self.draw_popups(f);
    }

    ///
    pub fn event(&mut self, ev: Event) {
        trace!("event: {:?}", ev);

        if self.check_quit(ev) {
            return;
        }

        let mut flags = NeedsUpdate::empty();

        if event_pump(ev, self.components_mut().as_mut_slice()) {
            flags.insert(NeedsUpdate::COMMANDS);
        } else if let Event::Key(k) = ev {
            let new_flags = match k {
                keys::TAB_TOGGLE => {
                    self.toggle_tabs();
                    NeedsUpdate::COMMANDS
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
            self.status_tab.update_diff();
        }
        if flags.contains(NeedsUpdate::COMMANDS) {
            self.update_commands();
        }
    }

    //TODO: do we need this?
    ///
    pub fn update(&mut self) {
        trace!("update");
        self.status_tab.update();
    }

    ///
    pub fn update_git(&mut self, ev: AsyncNotification) {
        trace!("update_git: {:?}", ev);

        self.status_tab.update_git(ev);

        match ev {
            AsyncNotification::Diff => (),
            AsyncNotification::Log => self.revlog.update(),
            //TODO: is that needed?
            AsyncNotification::Status => self.update_commands(),
        }
    }

    ///
    pub fn is_quit(&self) -> bool {
        self.do_quit
    }

    ///
    pub fn any_work_pending(&self) -> bool {
        self.status_tab.anything_pending()
            || self.revlog.any_work_pending()
    }
}

// private impls
impl App {
    accessors!(self, [msg, reset, commit, help, revlog, status_tab]);

    fn check_quit(&mut self, ev: Event) -> bool {
        if let Event::Key(e) = ev {
            if let keys::EXIT = e {
                self.do_quit = true;
                return true;
            }
        }
        false
    }

    fn toggle_tabs(&mut self) {
        self.tab += 1;
        self.tab %= 2;

        if self.tab == 1 {
            self.status_tab.hide();
            self.revlog.show();
        } else {
            self.status_tab.show();
            self.revlog.hide();
        }
    }

    fn update_commands(&mut self) {
        self.help.set_cmds(self.commands(true));
        self.current_commands = self.commands(false);
        self.current_commands.sort_by_key(|e| e.order);
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
                    )
                    .is_ok()
                    {
                        flags.insert(NeedsUpdate::ALL);
                    }
                } else if sync::reset_workdir_file(
                    CWD,
                    reset_item.path.as_str(),
                )
                .is_ok()
                {
                    flags.insert(NeedsUpdate::ALL);
                }
            }
            InternalEvent::ConfirmResetItem(reset_item) => {
                self.reset.open_for_path(reset_item);
                flags.insert(NeedsUpdate::COMMANDS);
            }
            InternalEvent::AddHunk(hash) => {
                if let Some((path, is_stage)) =
                    self.status_tab.selected_path()
                {
                    if is_stage {
                        if sync::unstage_hunk(CWD, path, hash)
                            .unwrap()
                        {
                            flags.insert(NeedsUpdate::ALL);
                        }
                    } else if sync::stage_hunk(CWD, path, hash)
                        .is_ok()
                    {
                        flags.insert(NeedsUpdate::ALL);
                    }
                }
            }
            InternalEvent::ShowMsg(msg) => {
                self.msg.show_msg(msg.as_str());
                flags.insert(NeedsUpdate::ALL);
            }
            InternalEvent::Update(u) => flags.insert(u),
            InternalEvent::OpenCommit => self.commit.show(),
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

        res.push(
            CommandInfo::new(
                commands::TOGGLE_TABS,
                true,
                !self.any_popup_visible(),
            )
            .hidden(),
        );

        res.push(
            CommandInfo::new(
                commands::QUIT,
                true,
                !self.any_popup_visible(),
            )
            .order(100),
        );

        res
    }

    fn any_popup_visible(&self) -> bool {
        self.commit.is_visible()
            || self.help.is_visible()
            || self.reset.is_visible()
            || self.msg.is_visible()
    }

    fn draw_popups<B: Backend>(&mut self, f: &mut Frame<B>) {
        let size = f.size();

        self.commit.draw(f, size);
        self.reset.draw(f, size);
        self.help.draw(f, size);
        self.msg.draw(f, size);
    }

    fn draw_tabs<B: Backend>(&self, f: &mut Frame<B>, r: Rect) {
        f.render_widget(
            Tabs::default()
                .block(Block::default().borders(Borders::BOTTOM))
                .titles(&[strings::TAB_STATUS, strings::TAB_LOG])
                .style(Style::default())
                .highlight_style(
                    self.theme
                        .tab(true)
                        .modifier(Modifier::UNDERLINED),
                )
                .divider(strings::TAB_DIVIDER)
                .select(self.tab),
            r,
        );
    }

    fn draw_commands<B: Backend>(
        f: &mut Frame<B>,
        r: Rect,
        cmds: &[CommandInfo],
        theme: Theme,
    ) {
        let splitter = Text::Styled(
            Cow::from(strings::CMD_SPLITTER),
            Style::default(),
        );

        let texts = cmds
            .iter()
            .filter_map(|c| {
                if c.show_in_quickbar() {
                    Some(Text::Styled(
                        Cow::from(c.text.name),
                        theme.toolbar(c.enabled),
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
}

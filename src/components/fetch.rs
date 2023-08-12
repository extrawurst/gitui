use crate::{
	components::{
		cred::CredComponent, visibility_blocking, CommandBlocking,
		CommandInfo, Component, DrawableComponent, EventState,
	},
	keys::SharedKeyConfig,
	queue::{InternalEvent, NeedsUpdate, Queue},
	strings,
	ui::{self, style::SharedTheme},
};
use anyhow::Result;
use asyncgit::{
	asyncjob::AsyncSingleJob,
	sync::{
		cred::{
			extract_username_password, need_username_password,
			BasicAuthCredential,
		},
		RepoPathRef,
	},
	AsyncFetchJob, AsyncGitNotification, ProgressPercent,
};
use crossbeam_channel::Sender;
use crossterm::event::Event;
use ratatui::{
	backend::Backend,
	layout::Rect,
	text::Span,
	widgets::{Block, BorderType, Borders, Clear, Gauge},
	Frame,
};

///
pub struct FetchComponent {
	repo: RepoPathRef,
	visible: bool,
	async_fetch: AsyncSingleJob<AsyncFetchJob>,
	progress: Option<ProgressPercent>,
	pending: bool,
	queue: Queue,
	theme: SharedTheme,
	key_config: SharedKeyConfig,
	input_cred: CredComponent,
}

impl FetchComponent {
	///
	pub fn new(
		repo: RepoPathRef,
		queue: &Queue,
		sender: &Sender<AsyncGitNotification>,
		theme: SharedTheme,
		key_config: SharedKeyConfig,
	) -> Self {
		Self {
			queue: queue.clone(),
			pending: false,
			visible: false,
			async_fetch: AsyncSingleJob::new(sender.clone()),
			progress: None,
			input_cred: CredComponent::new(
				theme.clone(),
				key_config.clone(),
			),
			theme,
			key_config,
			repo,
		}
	}

	///
	pub fn fetch(&mut self) -> Result<()> {
		self.show()?;
		if need_username_password(&self.repo.borrow())? {
			let cred = extract_username_password(&self.repo.borrow())
				.unwrap_or_else(|_| {
					BasicAuthCredential::new(None, None)
				});
			if cred.is_complete() {
				self.fetch_all(Some(cred));
			} else {
				self.input_cred.set_cred(cred);
				self.input_cred.show()?;
			}
		} else {
			self.fetch_all(None);
		}

		Ok(())
	}

	fn fetch_all(&mut self, cred: Option<BasicAuthCredential>) {
		self.pending = true;
		self.progress = None;
		self.progress = Some(ProgressPercent::empty());
		self.async_fetch.spawn(AsyncFetchJob::new(
			self.repo.borrow().clone(),
			cred,
		));
	}

	///
	pub const fn any_work_pending(&self) -> bool {
		self.pending
	}

	///
	pub fn update_git(&mut self, ev: AsyncGitNotification) {
		if self.is_visible() && ev == AsyncGitNotification::Fetch {
			self.update();
		}
	}

	///
	fn update(&mut self) {
		self.pending = self.async_fetch.is_pending();
		self.progress = self.async_fetch.progress();

		if !self.pending {
			self.hide();
			self.queue
				.push(InternalEvent::Update(NeedsUpdate::BRANCHES));
		}
	}
}

impl DrawableComponent for FetchComponent {
	fn draw<B: Backend>(
		&self,
		f: &mut Frame<B>,
		rect: Rect,
	) -> Result<()> {
		if self.visible {
			let progress = self.progress.unwrap_or_default().progress;

			let area = ui::centered_rect_absolute(30, 3, f.size());

			f.render_widget(Clear, area);
			f.render_widget(
				Gauge::default()
					.block(
						Block::default()
							.title(Span::styled(
								strings::FETCH_POPUP_MSG,
								self.theme.title(true),
							))
							.borders(Borders::ALL)
							.border_type(BorderType::Thick)
							.border_style(self.theme.block(true)),
					)
					.gauge_style(self.theme.push_gauge())
					.percent(u16::from(progress)),
				area,
			);
			self.input_cred.draw(f, rect)?;
		}

		Ok(())
	}
}

impl Component for FetchComponent {
	fn commands(
		&self,
		out: &mut Vec<CommandInfo>,
		force_all: bool,
	) -> CommandBlocking {
		if self.is_visible() || force_all {
			if !force_all {
				out.clear();
			}

			if self.input_cred.is_visible() {
				return self.input_cred.commands(out, force_all);
			}
			out.push(CommandInfo::new(
				strings::commands::close_msg(&self.key_config),
				!self.pending,
				self.visible,
			));
		}

		visibility_blocking(self)
	}

	fn event(&mut self, ev: &Event) -> Result<EventState> {
		if self.visible {
			if let Event::Key(_) = ev {
				if self.input_cred.is_visible() {
					self.input_cred.event(ev)?;

					if self.input_cred.get_cred().is_complete()
						|| !self.input_cred.is_visible()
					{
						self.fetch_all(Some(
							self.input_cred.get_cred().clone(),
						));
						self.input_cred.hide();
					}
				}
			}
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

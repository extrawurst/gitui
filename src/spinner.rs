use std::io;
use tui::{backend::Backend, buffer::Cell, Terminal};

static SPINNER_CHARS: &[char] = &['|', '/', '-', '\\'];

///
#[derive(Default)]
pub struct Spinner {
    idx: usize,
    pending: bool,
}

impl Spinner {
    /// increment spinner graphic by one
    pub fn update(&mut self) {
        self.idx += 1;
        self.idx %= SPINNER_CHARS.len();
    }

    ///
    pub fn set_state(&mut self, pending: bool) {
        self.pending = pending;
    }

    /// draws or removes spinner char depending on `pending` state
    pub fn draw<B: Backend>(
        &self,
        terminal: &mut Terminal<B>,
    ) -> io::Result<()> {
        let idx = self.idx;

        let c: Cell = Cell::default()
            .set_char(if self.pending {
                SPINNER_CHARS[idx]
            } else {
                ' '
            })
            .clone();
        terminal
            .backend_mut()
            .draw(vec![(0_u16, 0_u16, &c)].into_iter())?;
        tui::backend::Backend::flush(terminal.backend_mut())?;

        Ok(())
    }
}

use std::io;
use tui::{backend::Backend, buffer::Cell, Terminal};

static SPINNER_CHARS: &[char] = &['|', '/', '-', '\\'];

pub struct Spinner {
    idx: usize,
}

impl Spinner {
    /// increment spinner graphic by one
    pub fn update(&mut self) {
        self.idx += 1;
        self.idx %= SPINNER_CHARS.len();
    }

    pub fn new() -> Self {
        Self { idx: 0 }
    }

    /// draws or removes spinner char depending on `pending` state
    pub fn draw<B: Backend>(
        &self,
        terminal: &mut Terminal<B>,
        pending: bool,
    ) -> io::Result<()> {
        let idx = self.idx;

        let c: Cell = Cell::default()
            .set_char(if pending { SPINNER_CHARS[idx] } else { ' ' })
            .clone();
        terminal
            .backend_mut()
            .draw(vec![(0_u16, 0_u16, &c)].into_iter())?;
        tui::backend::Backend::flush(terminal.backend_mut())?;

        Ok(())
    }
}

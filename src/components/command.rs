///
#[derive(Copy, Clone)]
pub struct CommandText {
    ///
    pub name: &'static str,
    ///
    pub desc: &'static str,
    ///
    pub group: &'static str,
}

impl CommandText {
    pub const fn new(
        name: &'static str,
        desc: &'static str,
        group: &'static str,
    ) -> Self {
        Self { name, desc, group }
    }
}

///
pub struct CommandInfo {
    ///
    pub text: CommandText,
    /// available but not active in the context
    pub enabled: bool,
    /// will show up in the quick bar
    pub quick_bar: bool,
    /// available in current app state
    pub available: bool,
    /// used to order commands in quickbar
    pub order: i8,
}

impl CommandInfo {
    ///
    pub fn new(
        text: CommandText,
        enabled: bool,
        available: bool,
    ) -> Self {
        Self {
            text,
            enabled,
            quick_bar: true,
            available,
            order: 0,
        }
    }
    ///
    pub fn order(self, order: i8) -> Self {
        let mut res = self;
        res.order = order;
        res
    }
    ///
    pub fn hidden(self) -> Self {
        let mut res = self;
        res.quick_bar = false;
        res
    }
    ///
    pub fn print(&self, out: &mut String) {
        out.push_str(self.text.name);
    }
    ///
    pub fn show_in_quickbar(&self) -> bool {
        self.quick_bar && self.available
    }
}

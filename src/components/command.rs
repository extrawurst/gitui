///
pub struct CommandInfo {
    ///
    pub name: String,
    ///
    // pub keys:
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
    pub fn new(name: &str, enabled: bool, available: bool) -> Self {
        Self {
            name: name.to_string(),
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
        out.push_str(self.name.as_str());
    }
    ///
    pub fn show_in_quickbar(&self) -> bool {
        self.quick_bar && self.available
    }
}

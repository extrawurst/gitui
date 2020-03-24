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
}

impl CommandInfo {
    ///
    pub fn new(name: &str, enabled: bool, available: bool) -> Self {
        Self {
            name: name.to_string(),
            enabled,
            quick_bar: true,
            available,
        }
    }
    ///
    pub fn new_hidden(
        name: &str,
        enabled: bool,
        available: bool,
    ) -> Self {
        Self {
            name: name.to_string(),
            enabled,
            quick_bar: false,
            available,
        }
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

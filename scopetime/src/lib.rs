use log::trace;
use std::time::Instant;

///
pub struct ScopeTimeLog<'a> {
    title: &'a str,
    mod_path: &'a str,
    file: &'a str,
    line: u32,
    time: Instant,
}

///
impl<'a> ScopeTimeLog<'a> {
    ///
    pub fn new(
        mod_path: &'a str,
        title: &'a str,
        file: &'a str,
        line: u32,
    ) -> Self {
        Self {
            title,
            mod_path,
            file,
            line,
            time: Instant::now(),
        }
    }
}

impl<'a> Drop for ScopeTimeLog<'a> {
    fn drop(&mut self) {
        trace!(
            "scopetime: {:?} ms [{}::{}] @{}:{}",
            self.time.elapsed().as_millis(),
            self.mod_path,
            self.title,
            self.file,
            self.line,
        );
    }
}

#[cfg(feature = "enabled")]
#[macro_export]
macro_rules! scope_time {
    ($target:literal) => {
        //TODO: add module_path!() aswell?
        #[allow(unused_variables)]
        let time = $crate::ScopeTimeLog::new(
            module_path!(),
            $target,
            file!(),
            line!(),
        );
    };
}

#[cfg(not(feature = "enabled"))]
#[macro_export]
macro_rules! scope_time {
    ($target:literal) => {};
}

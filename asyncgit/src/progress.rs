//!

use std::cmp;

use easy_cast::CastFloat;

///
#[derive(Clone, Debug)]
pub struct ProgressPercent {
    /// percent 0..100
    pub progress: u8,
}

impl ProgressPercent {
    ///
    pub fn new(current: usize, total: usize) -> Self {
        let total = cmp::max(current, total) as f32;
        let progress = current as f32 / total * 100.0;
        let progress = progress.cast_nearest();
        Self { progress }
    }
    ///
    pub const fn empty() -> Self {
        Self { progress: 0 }
    }
    ///
    pub const fn full() -> Self {
        Self { progress: 100 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_zero_total() {
        let prog = ProgressPercent::new(1, 0);

        assert_eq!(prog.progress, 100);
    }

    #[test]
    fn test_progress_rounding() {
        let prog = ProgressPercent::new(2, 10);

        assert_eq!(prog.progress, 20);
    }
}

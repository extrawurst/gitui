/// helper struct to not pollute main with feature flags shenanigans for the profiler
/// also we make sure to generate a flamegraph on program exit
pub struct Profiler {
	#[cfg(feature = "pprof")]
	#[cfg(not(windows))]
	guard: pprof::ProfilerGuard<'static>,
}

impl Profiler {
	#[allow(clippy::missing_const_for_fn)]
	pub fn new() -> Self {
		Self {
			#[cfg(feature = "pprof")]
			#[cfg(not(windows))]
			guard: pprof::ProfilerGuard::new(100)
				.expect("profiler launch error"),
		}
	}

	#[allow(clippy::unused_self)]
	fn report(&mut self) {
		#[cfg(feature = "pprof")]
		#[cfg(not(windows))]
		if let Ok(report) = self.guard.report().build() {
			let file = std::fs::File::create("flamegraph.svg")
				.expect("flamegraph file err");

			report.flamegraph(&file).expect("flamegraph write err");

			log::info!("profiler reported");
		}
	}
}

impl Drop for Profiler {
	fn drop(&mut self) {
		self.report();
	}
}

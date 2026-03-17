use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

/// Progress reporter for analysis stages.
pub struct AnalysisProgress {
    multi: MultiProgress,
    enabled: bool,
}

impl AnalysisProgress {
    /// Create a new progress reporter.
    pub fn new(enabled: bool) -> Self {
        Self {
            multi: MultiProgress::new(),
            enabled,
        }
    }

    /// Create a spinner for a stage.
    pub fn stage_spinner(&self, message: &str) -> ProgressBar {
        if !self.enabled {
            return ProgressBar::hidden();
        }

        let pb = self.multi.add(ProgressBar::new_spinner());
        pb.set_style(
            ProgressStyle::with_template("{spinner:.cyan} {msg}")
                .unwrap()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ "),
        );
        pb.set_message(message.to_string());
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        pb
    }

    /// Create a progress bar for file processing.
    pub fn file_progress(&self, total: u64, message: &str) -> ProgressBar {
        if !self.enabled {
            return ProgressBar::hidden();
        }

        let pb = self.multi.add(ProgressBar::new(total));
        pb.set_style(
            ProgressStyle::with_template(
                "{spinner:.cyan} {msg} [{bar:30.cyan/dim}] {pos}/{len} ({eta})",
            )
            .unwrap()
            .progress_chars("━━╸━"),
        );
        pb.set_message(message.to_string());
        pb
    }

    /// Finish all progress bars.
    pub fn finish(&self) {
        self.multi.clear().ok();
    }
}

impl Default for AnalysisProgress {
    fn default() -> Self {
        Self::new(false)
    }
}

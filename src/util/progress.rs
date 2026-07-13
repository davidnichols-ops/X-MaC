use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

#[allow(dead_code)]
pub struct ProgressReporter {
    quiet: bool,
    multi_progress: Option<MultiProgress>,
}

#[allow(dead_code)]
impl ProgressReporter {
    pub fn new(quiet: bool) -> Self {
        Self {
            quiet,
            multi_progress: if quiet {
                None
            } else {
                Some(MultiProgress::new())
            },
        }
    }

    pub fn create_bar(&self, len: u64, message: &str) -> Option<ProgressBar> {
        if self.quiet {
            return None;
        }

        let pb = ProgressBar::new(len);
        pb.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}",
                )
                .unwrap()
                .progress_chars("#>-"),
        );
        pb.set_message(message.to_string());

        if let Some(ref mp) = self.multi_progress {
            mp.add(pb.clone());
        }

        Some(pb)
    }

    pub fn create_spinner(&self, message: &str) -> Option<ProgressBar> {
        if self.quiet {
            return None;
        }

        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        pb.set_message(message.to_string());

        Some(pb)
    }
}

impl Default for ProgressReporter {
    fn default() -> Self {
        Self::new(false)
    }
}

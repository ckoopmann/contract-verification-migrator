use crate::verification::VerificationResult;
use console::style;
use eyre::Result;
use indicatif::{MultiProgress, MultiProgressAlignment, ProgressBar, ProgressStyle};
use std::sync::Arc;
use std::time::Duration;

pub fn initialize_multi_progress(progress_bar: bool) -> Option<Arc<MultiProgress>> {
    if progress_bar {
        let mp = Arc::new(MultiProgress::new());
        mp.set_alignment(MultiProgressAlignment::Top);
        Some(mp)
    } else {
        None
    }
}

pub fn initialize_progress_bar(
    mp: Option<Arc<MultiProgress>>,
    contract_address: &String,
) -> Option<ProgressBar> {
    if let Some(mp) = mp.clone() {
        let pb = mp.add(ProgressBar::new_spinner());
        pb.enable_steady_tick(Duration::from_millis(120));
        pb.set_style(ProgressStyle::with_template("{prefix}{msg}{spinner:.yellow} ").unwrap());
        pb.set_prefix(format!("{} - ", contract_address));
        pb.set_message(format!("{}", style("Copying ").yellow()));
        Some(pb)
    } else {
        None
    }
}

pub fn update_progress_bar(pb: Option<ProgressBar>, result: &Result<VerificationResult>) {
    if let Some(pb) = pb {
        match result {
            Ok(VerificationResult::Success) => {
                pb.finish_with_message(format!("{}", style("Success ✔").green(),));
            }
            Ok(VerificationResult::AlreadyVerified) => {
                pb.finish_with_message(format!("{}", style("Already Verified ✔").green(),));
            }
            Err(err) => {
                pb.finish_with_message(format!("{}", style(format!("Error: {}", err)).red(),));
            }
        }
    }
}

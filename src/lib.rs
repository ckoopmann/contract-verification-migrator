use eyre::Result;
use futures::future::FutureExt;

mod verification;
pub use verification::{copy_etherscan_verification_for_contract, VerificationResult};
mod progress_bar;
use progress_bar::{initialize_multi_progress, initialize_progress_bar, update_progress_bar};

pub async fn copy_etherscan_verification(
    contract_addresses: Vec<String>,
    source_api_key: String,
    source_url: String,
    target_api_key: String,
    target_url: String,
    progress_bar: bool,
) -> Vec<Result<VerificationResult>> {
    let mp = initialize_multi_progress(progress_bar);
    let tasks: Vec<_> = contract_addresses
        .into_iter()
        .map(move |contract_address| {
            let pb = initialize_progress_bar(mp.clone(), &contract_address);
            copy_etherscan_verification_for_contract(
                contract_address.clone(),
                source_api_key.clone(),
                source_url.clone(),
                target_api_key.clone(),
                target_url.clone(),
            )
            .then(move |result| {
                update_progress_bar(pb, &result);
                futures::future::ready(result)
            })
        })
        .collect();
    return futures::future::join_all(tasks).await;
}

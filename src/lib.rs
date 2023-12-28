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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_copy_verification_from_etherscan_to_blockscout() {
        let source_api_key = std::env::var("ETHERSCAN_API_KEY").expect("ETHERSCAN_API_KEY not set");
        let source_url = "https://api.etherscan.io/api".to_string();
        let target_api_key = std::env::var("BLOCKSCOUT_API_KEY").expect("BLOCKSCOUT_API_KEY not set");
        let target_url = "https://eth.blockscout.com/api".to_string();
        let contract_addresses = vec![  
            "0x341c05c0E9b33C0E38d64de76516b2Ce970bB3BE".to_string(),
            "0x7C07F7aBe10CE8e33DC6C5aD68FE033085256A84".to_string(),
        ];
        let results = copy_etherscan_verification(
            contract_addresses,
            source_api_key,
            source_url,
            target_api_key,
            target_url,
            false,
        ).await;
        assert!(!results.into_iter().any(|result| result.is_err()));
    }
}

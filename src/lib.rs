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
    futures::future::join_all(tasks).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_copy_verification_from_etherscan_to_blockscout() {
        let results = copy_etherscan_verification(
            contract_addresses(),
            etherscan_api_key(),
            etherscan_url(),
            blockscout_api_key(),
            blockscout_url(),
            false,
        )
        .await;
        assert!(!results.into_iter().any(|result| result.is_err()));
    }

    #[tokio::test]
    async fn test_copy_verification_from_blockscout_to_etherscan() {
        let results = copy_etherscan_verification(
            contract_addresses(),
            blockscout_api_key(),
            blockscout_url(),
            etherscan_api_key(),
            etherscan_url(),
            false,
        )
        .await;
        assert!(!results.into_iter().any(|result| result.is_err()));
    }

    // Complex contract verified in "standard-solidity-json" format (non-flattened)
    const UNI_V3_ROUTER: &str = "0xE592427A0AEce92De3Edee1F18E0157C05861564";
    // Complex contract verified in flattened format on etherscan
    const ICETH_TOKEN: &str = "0x7C07F7aBe10CE8e33DC6C5aD68FE033085256A84";
    // Complex proxy contract - note that this should only attempt to migrate / copy the
    // verification of the proxy logic itself (not the implementations)
    const ZEROEX_PROXY: &str = "0xDef1C0ded9bec7F1a1670819833240f027b25EfF";

    fn contract_addresses() -> Vec<String> {
        vec![ICETH_TOKEN.to_string(), UNI_V3_ROUTER.to_string(), ZEROEX_PROXY.to_string()]
    }
    fn etherscan_url() -> String {
        "https://api.etherscan.io/api".to_string()
    }
    fn etherscan_api_key() -> String {
        std::env::var("ETHERSCAN_API_KEY").expect("ETHERSCAN_API_KEY not set")
    }

    fn blockscout_url() -> String {
        "https://eth.blockscout.com/api".to_string()
    }

    fn blockscout_api_key() -> String {
        std::env::var("BLOCKSCOUT_API_KEY").expect("BLOCKSCOUT_API_KEY not set")
    }
}

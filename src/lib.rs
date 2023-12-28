//! Copy contract verification from one block-explorer to another
//!
//! This crate allows you to easily copy the verfied source code from one block-explorer to another.
//! It reads the source code / metadata from the source_url and submits it to the target_url.
//! This assumes that both block-explorers are compatible with the the etherscan api specification
//!
//! ```rust
//!    let results = contract_verification_migrator::copy_etherscan_verification(
//!        vec!["0xE592427A0AEce92De3Edee1F18E0157C05861564".to_string()],
//!        "<YOUR_ETHERSCAN_API_KEY>".to_string(),
//!        "https://api.etherscan.io/api".to_string(),
//!        "<YOUR_BLOCKSCOUT_API_KEY>".to_string(),
//!        "https://eth.blockscout.com/api".to_string(),
//!        true,
//!    );
//! ```
#![warn(missing_docs)]

use eyre::Result;
use futures::future::FutureExt;

mod verification;
pub use verification::{copy_etherscan_verification_for_contract, VerificationResult};
mod progress_bar;
use progress_bar::{initialize_multi_progress, initialize_progress_bar, update_progress_bar};

/// Copy contract verification of multiple contracts from one block-explorer to another
///
/// # Arguments
/// - `contract_addresses` - Vector of contract addresses for which to copy the contract
/// verification
/// - `source_api_key` - The api key for the source block-explorer's api
/// - `source_url` - The url of the source block-explorer's api
/// - `target_api_key` - The api key for the target block-explorer's api
/// - `target_url` - The url of the target block-explorer's api
/// - `progress_bar` - Boolean indicating wether or not to display progress bars for the individual
/// requests
///
/// # Examples
///
/// ```rust
///    let results = contract_verification_migrator::copy_etherscan_verification(
///        vec!["0xE592427A0AEce92De3Edee1F18E0157C05861564".to_string()],
///        "<YOUR_ETHERSCAN_API_KEY>".to_string(),
///        "https://api.etherscan.io/api".to_string(),
///        "<YOUR_BLOCKSCOUT_API_KEY>".to_string(),
///        "https://eth.blockscout.com/api".to_string(),
///        true,
///     );
///
/// ```
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
    // Proxy contract - note that this should only attempt to migrate / copy the proxy itself
    // and not the implementation contract
    const BLUR: &str = "0x000000000000Ad05Ccc4F10045630fb830B95127";

    fn contract_addresses() -> Vec<String> {
        vec![
            UNI_V3_ROUTER.to_string(),
            ICETH_TOKEN.to_string(),
            BLUR.to_string(),
        ]
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

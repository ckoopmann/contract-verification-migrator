use eyre::eyre;
use eyre::{Context, Result};
use foundry_block_explorers::contract::{
    Metadata, SourceCodeEntry, SourceCodeLanguage, SourceCodeMetadata,
};
use foundry_block_explorers::verify::{CodeFormat, VerifyContract};
use foundry_block_explorers::Client;
use serde_json::json;
use std::collections::HashMap;

#[derive(Debug)]
/// Enum containing different verification outcomes that result in the contract being subsequently
/// verified on the target block-explorer
/// Note that failure / errors are accounted for by wrapping this enum in a  standard Result
pub enum VerificationResult {
    /// Indicates successful verification of previously unverified contract
    Success,
    /// Indicates that the given contract had been verified already
    AlreadyVerified,
}

enum VerificationRequestResponse {
    Submitted(String),
    AlreadyVerified,
}

/// Copy contract verification of a single contract from one block-explorer to another
///
/// # Arguments
/// - `contract_address` - The contract address for which to copy the source code verification
/// verification
/// - `source_api_key` - The api key for the source block-explorer's api
/// - `source_url` - The url of the source block-explorer's api
/// - `target_api_key` - The api key for the target block-explorer's api
/// - `target_url` - The url of the target block-explorer's api
///
/// # Examples
///
/// ```rust
///    let results = contract_verification_migrator::copy_etherscan_verification_for_contract(
///        "0xE592427A0AEce92De3Edee1F18E0157C05861564".to_string(),
///        "<YOUR_ETHERSCAN_API_KEY>".to_string(),
///        "https://api.etherscan.io/api".to_string(),
///        "<YOUR_BLOCKSCOUT_API_KEY>".to_string(),
///        "https://eth.blockscout.com/api".to_string(),
///     );
///
/// ```

pub async fn copy_etherscan_verification_for_contract(
    contract_address: String,
    source_api_key: String,
    source_url: String,
    target_api_key: String,
    target_url: String,
) -> Result<VerificationResult> {
    let source_client = Client::builder()
        .with_api_key(source_api_key)
        .with_url(source_url.clone())?
        .with_api_url(source_url)?
        .build()?;
    let target_client = Client::builder()
        .with_api_key(target_api_key)
        .with_url(target_url.clone())?
        .with_api_url(target_url)?
        .build()?;
    let metadata = source_client
        .contract_source_code(contract_address.parse()?)
        .await?
        .items[0]
        .clone();
    let verification_request =
        convert_metadata_to_verification_request(&contract_address, &metadata)?;
    let verification_response =
        send_verification_request(verification_request, &target_client).await?;
    match verification_response {
        VerificationRequestResponse::Submitted(id) => {
            await_contract_verification(id, &target_client).await
        }
        VerificationRequestResponse::AlreadyVerified => Ok(VerificationResult::AlreadyVerified),
    }
}

fn convert_metadata_to_verification_request(
    contract_address: &str,
    metadata: &Metadata,
) -> Result<VerifyContract> {
    let contract_name = format!("{}.sol:{}", metadata.contract_name, metadata.contract_name);
    let source = match metadata.source_code {
        // Blockscout does not accept "single-file" source code for verificatin so we convert it
        // into "solidity-standard-json-input" format
        SourceCodeMetadata::SourceCode(..) => {
            let mut source_code_entries: HashMap<String, SourceCodeEntry> = HashMap::new();
            source_code_entries.insert(
                contract_name.clone(),
                SourceCodeEntry {
                    content: metadata.source_code(),
                },
            );
            let source_code = SourceCodeMetadata::Metadata {
                language: Some(SourceCodeLanguage::Solidity),
                settings: Some(json!( {
                    "evm_version": metadata.evm_version,
                    "libraries": {},
                    "optimizer": {
                        "enabled": metadata.optimization_used == 1,
                        "runs": metadata.runs,
                    },
                    "remappings": [],
                })),
                sources: source_code_entries,
            };
            serde_json::to_string(&source_code)?
        }
        SourceCodeMetadata::Metadata { .. } => serde_json::to_string(&metadata.source_code)?,
        // Note: This case is untested
        SourceCodeMetadata::Sources(_) => serde_json::to_string(&metadata.source_code)?,
    };
    // if compiler version does not start with a "v" add it
    let mut compiler_version = metadata.compiler_version.clone();

    // Apparently sometimes Blockscout omits the leading v in the contract version
    if !compiler_version.starts_with('v') {
        compiler_version.insert(0, 'v');
    };

    let verification_request = VerifyContract {
        address: contract_address.parse()?,
        code_format: CodeFormat::StandardJsonInput,
        contract_name: contract_name.clone(),
        compiler_version,
        runs: Some(metadata.runs.to_string()),
        optimization_used: Some(metadata.optimization_used.to_string()),
        constructor_arguments: Some(hex::encode(metadata.constructor_arguments.clone())),
        blockscout_constructor_arguments: Some(hex::encode(metadata.constructor_arguments.clone())),
        evm_version: Some(metadata.evm_version.clone()),
        source,
        other: std::collections::HashMap::new(),
    };
    Ok(verification_request)
}

async fn send_verification_request(
    verification_request: VerifyContract,
    target_client: &Client,
) -> Result<VerificationRequestResponse> {
    let verification_response = target_client
        .submit_contract_verification(&verification_request)
        .await?;
    if verification_response.message != "OK" {
        if verification_response
            .result
            .to_lowercase()
            .contains("already verified")
        {
            return Ok(VerificationRequestResponse::AlreadyVerified);
        }
        return Err(eyre::eyre!(
            "Verification returned non-ok response: {}",
            verification_response.result
        ));
    }
    Ok(VerificationRequestResponse::Submitted(
        verification_response.result,
    ))
}

async fn await_contract_verification(
    id: String,
    target_client: &Client,
) -> Result<VerificationResult> {
    let max_verification_status_retries = 10;
    let interval = std::time::Duration::from_secs(10);
    for _ in 0..max_verification_status_retries {
        let resp = target_client
            .check_contract_verification_status(id.clone())
            .await
            .wrap_err("Failed to request verification status")?;

        if resp.result.contains("Unable to verify") {
            return Err(eyre!("Unable to verify.",));
        }

        if resp.result == "Already Verified" {
            return Ok(VerificationResult::AlreadyVerified);
        }

        if resp.result == "Pass - Verified" {
            return Ok(VerificationResult::Success);
        }

        // Wait for interval before checking again
        tokio::time::sleep(interval).await;
    }
    Err(eyre!("Verification timed out"))
}

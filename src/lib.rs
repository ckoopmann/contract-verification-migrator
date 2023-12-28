use eyre::eyre;
use eyre::{Context, Result};
use foundry_block_explorers::contract::{
    Metadata, SourceCodeEntry, SourceCodeLanguage, SourceCodeMetadata,
};
use foundry_block_explorers::verify::{CodeFormat, VerifyContract};
use foundry_block_explorers::Client;
use serde_json::json;
use std::collections::HashMap;

pub async fn copy_etherscan_verification(
    contract_addresses: Vec<String>,
    source_api_key: String,
    source_url: String,
    target_api_key: String,
    target_url: String,
) -> Result<()> {
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
    for contract_address in contract_addresses {
        copy_etherscan_verification_for_contract(&contract_address, &source_client, &target_client)
            .await?;
    }
    Ok(())
}

async fn copy_etherscan_verification_for_contract(
    contract_address: &String,
    source_client: &Client,
    target_client: &Client,
) -> Result<()> {
    let metadata = source_client
        .contract_source_code(contract_address.parse()?)
        .await?
        .items[0]
        .clone();
    let verification_request =
        convert_metadata_to_verification_request(contract_address, &metadata)?;
    let id = send_verification_request(verification_request, &target_client).await?;
    await_contract_verification(id, &target_client).await
}

fn convert_metadata_to_verification_request(
    contract_address: &String,
    metadata: &Metadata,
) -> Result<VerifyContract> {
    let source = match metadata.source_code {
        // Blockscout does not accept "single-file" source code for verificatin so we convert it
        // into "solidity-standard-json-input" format
        SourceCodeMetadata::SourceCode(..) => {
            let mut source_code_entries: HashMap<String, SourceCodeEntry> = HashMap::new();
            source_code_entries.insert(
                metadata.contract_name.clone(),
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
                        "enabled": if metadata.optimization_used == 1 { true } else { false },
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
    let verification_request = VerifyContract {
        address: contract_address.parse()?,
        code_format: CodeFormat::StandardJsonInput,
        contract_name: metadata.contract_name.clone(),
        compiler_version: metadata.compiler_version.clone(),
        runs: Some(metadata.runs.to_string()),
        optimization_used: Some(metadata.optimization_used.to_string()),
        constructor_arguments: Some(hex::encode(metadata.constructor_arguments.clone())),
        blockscout_constructor_arguments: Some(hex::encode(metadata.constructor_arguments.clone())),
        evm_version: Some(metadata.evm_version.clone()),
        source,
        other: std::collections::HashMap::new(),
    };
    return Ok(verification_request);
}

async fn send_verification_request(
    verification_request: VerifyContract,
    target_client: &Client,
) -> Result<String> {
    let verification_response = target_client
        .submit_contract_verification(&verification_request)
        .await?;
    if verification_response.message != "OK" {
        return Err(eyre::eyre!(
            "Verification returned non-ok response: {}",
            verification_response.message
        ));
    }
    Ok(verification_response.result)
}

async fn await_contract_verification(id: String, target_client: &Client) -> Result<()> {
    let max_verification_status_retries = 10;
    let interval = std::time::Duration::from_secs(10);
    for _ in 0..max_verification_status_retries {
        println!("Checking verification status with id: {}", id);
        let resp = target_client
            .check_contract_verification_status(id.clone())
            .await
            .wrap_err("Failed to request verification status")?;
        eprintln!("Contract verification status:\nResponse: {:#?}", resp);

        if resp.result.contains("Unable to verify") {
            return Err(eyre!("Unable to verify.",));
        }

        if resp.result == "Already Verified" {
            println!("Contract source code already verified");
            return Ok(());
        }

        if resp.status == "0" {
            println!("Contract failed to verify.");
            std::process::exit(1);
        }

        if resp.result == "Pass - Verified" {
            println!("Contract successfully verified");
            return Ok(());
        }

        // Wait for interval before checking again
        tokio::time::sleep(interval).await;
    }
    Err(eyre!("Verification timed out"))
}

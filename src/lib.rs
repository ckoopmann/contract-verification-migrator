use console::style;
use eyre::eyre;
use eyre::{Context, Result};
use foundry_block_explorers::contract::{
    Metadata, SourceCodeEntry, SourceCodeLanguage, SourceCodeMetadata,
};
use foundry_block_explorers::verify::{CodeFormat, VerifyContract};
use foundry_block_explorers::Client;
use futures::FutureExt;
use indicatif::{MultiProgress, MultiProgressAlignment, ProgressBar, ProgressStyle};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug)]
pub enum VerificationResult {
    Success,
    AlreadyVerified,
}

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
                update_progress_bar(pb, contract_address, &result);
                futures::future::ready(result)
            })
        })
        .collect();
    return futures::future::join_all(tasks).await;
}

async fn copy_etherscan_verification_for_contract(
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

        if resp.status == "0" {
            return Err(eyre!("Contract failed to verify.",));
        }

        if resp.result == "Pass - Verified" {
            return Ok(VerificationResult::Success);
        }

        // Wait for interval before checking again
        tokio::time::sleep(interval).await;
    }
    Err(eyre!("Verification timed out"))
}

fn initialize_multi_progress(progress_bar: bool) -> Option<Arc<MultiProgress>> {
    if progress_bar {
        let mp = Arc::new(MultiProgress::new());
        mp.set_alignment(MultiProgressAlignment::Bottom);
        Some(mp)
    } else {
        None
    }
}

fn initialize_progress_bar(
    mp: Option<Arc<MultiProgress>>,
    contract_address: &String,
) -> Option<ProgressBar> {
    if let Some(mp) = mp.clone() {
        let pb = mp.add(ProgressBar::new_spinner());
        pb.enable_steady_tick(Duration::from_millis(120));
        pb.set_style(
            ProgressStyle::with_template("{msg}{spinner:.yellow} ")
                .unwrap()
        );
        pb.set_message(format!(
                        "{}: {}",
                        contract_address,
                        style("Copying ").yellow(),
                    ));
        Some(pb)
    } else {
        None
    }
}

fn update_progress_bar(
    pb: Option<ProgressBar>,
    contract_address: String,
    result: &Result<VerificationResult>,
) {
    if let Some(pb) = pb {
        let style_finished = ProgressStyle::with_template("{prefix}{msg}").unwrap();
                pb.set_style(style_finished.clone());
        match result {
            Ok(VerificationResult::Success) => {
                    pb.finish_with_message(format!(
                        "{} - {}",
                        contract_address,
                        style("Success ✔").green(),
                    ));
            }
            Ok(VerificationResult::AlreadyVerified) => {
                    pb.finish_with_message(format!(
                        "{} - {}",
                        contract_address,
                        style("Already Verified ✔").green(),
                    ));
            }
            Err(ref err) => {
                    pb.finish_with_message(format!(
                        "{} - {}",
                        contract_address,
                        style(format!("Error: {}", err)).red(),
                    ));
            }
        }
    }
}

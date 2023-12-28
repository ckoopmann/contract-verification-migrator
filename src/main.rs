use clap::Parser;

/// Decode transaction calldata without abi
#[derive(Parser, Debug)]
#[clap(name = "contract_verfication_migrator")]
struct Args {
    /// The contract's address.
    addresses: Vec<String>,

    #[clap(long)]
    source_url: String,
    #[clap(long)]
    source_api_key: String,
    #[clap(long)]
    target_url: String,
    #[clap(long)]
    target_api_key: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    contract_verification_migrator::copy_etherscan_verification(
        args.addresses,
        args.source_api_key,
        args.source_url,
        args.target_api_key,
        args.target_url,
    )
    .await
    .unwrap();
}

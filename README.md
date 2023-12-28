# Contract Verification Migrator
Rust library / binary to easily copy / migrate contract verifications across block explorers

## How it works
For each specified contract the library will download the source code and metadata from the source explorer and submit it to the target explorer.
This assumes that both explorers api adhere to the the etherscan api specifications.

## How to use as binary:
1. Install: `cargo install contract-verification-migrator`
2. Run: `contract-verification-migrator --source-url https://api.etherscan.io/api --source-api-key <YOUR_ETHERSCAN_API_KEY> --target-url https://eth.blockscout.com/api --target-api-key <BLOCKSCOUT_API_KEY> 0x341c05c0E9b33C0E38d64de76516b2Ce970bB3BE 0x7C07F7aBe10CE8e33DC6C5aD68FE033085256A84`


## How to use as library:
1. Install: `cargo add contract-verification-migrator`
2. Import: 
```rust
    let results = contract_verification_migrator::copy_etherscan_verification(
        vec!["0xE592427A0AEce92De3Edee1F18E0157C05861564".to_string()],
        "<YOUR_ETHERSCAN_API_KEY>".to_string(),
        "https://api.etherscan.io/api".to_string(),
        "<YOUR_BLOCKSCOUT_API_KEY>".to_string(),
        "https://eth.blockscout.com/api".to_string(),
        true,
     );
 ```


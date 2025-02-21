use anyhow::Result;
use bincode;
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::{self, RpcClient};
use solana_sdk::{
    instruction::Instruction,
    hash::{Hash, Hasher},
    native_token::LAMPORTS_PER_SOL,
    signature::Signature,
    signer::{self, Signer},
    system_instruction, system_program,
    transaction::Transaction,
};
use solana_transaction_status::UiTransactionEncoding::{self, Binary};
use std::{collections::HashMap, str::FromStr};
// use serde_json;

#[derive(Serialize, Deserialize, Debug)]
struct RollupTransaction {
    sender: String,
    sol_transaction: Transaction,
    keypair_bytes: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetTransaction {
    pub get_tx: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "status", content = "data")]
pub enum TransactionResponse {
    Success { message: String },
    Error { message: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    let keypair = signer::keypair::read_keypair_file("./mykey_1.json").unwrap();
    let keypair2 = signer::keypair::read_keypair_file("./testkey.json").unwrap();
    let rpc_client = RpcClient::new("https://api.devnet.solana.com".into());
    let client = reqwest::Client::new();

    println!("\nTesting delegation flow...");

    // 1. Create a transfer transaction
    let transfer_amount = LAMPORTS_PER_SOL/4;
    let ix = system_instruction::transfer(
        &keypair2.pubkey(), 
        &keypair.pubkey(), 
        transfer_amount
    );
    
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&keypair2.pubkey()),
        &[&keypair2],
        rpc_client.get_latest_blockhash().await?,
    );

    // 2. Submit transaction to rollup
    let rtx = RollupTransaction {
        sender: keypair2.pubkey().to_string(),
        sol_transaction: tx,
        keypair_bytes: keypair2.to_bytes().to_vec(),
    };

    let response = client
        .post("http://127.0.0.1:8080/submit_transaction")
        .json(&rtx)
        .send()
        .await?
        .json::<TransactionResponse>()
        .await?;

    // 3. Handle response
    match response {
        TransactionResponse::Success { message } => {
            println!("Success: {}", message);
        },
        TransactionResponse::Error { message } => {
            println!("Error: {}", message);
        }
    }

    Ok(())
}

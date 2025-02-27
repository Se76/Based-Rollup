use anyhow::Result;
use bincode;
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::{self, RpcClient};
use solana_sdk::{
    instruction::Instruction,
    hash::{Hash, Hasher},
    native_token::LAMPORTS_PER_SOL,
    signature::{Keypair, Signer},
    system_instruction, system_program,
    transaction::Transaction,
    pubkey::Pubkey,
};
use solana_transaction_status::UiTransactionEncoding::{self, Binary};
use std::{collections::HashMap, str::FromStr, time::Duration, fs};
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
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load existing keypairs from files
    let sender = Keypair::from_bytes(&fs::read("keys/sender.json")?)?;
    let receiver = Keypair::from_bytes(&fs::read("keys/receiver.json")?)?;
    
    // Connect to devnet
    let rpc_client = RpcClient::new("https://api.devnet.solana.com".to_string());
    
    // Print initial balances
    let sender_balance = rpc_client.get_balance(&sender.pubkey()).await?;
    let receiver_balance = rpc_client.get_balance(&receiver.pubkey()).await?;
    println!("Initial Sender {} balance: {} SOL", sender.pubkey(), sender_balance as f64 / 1_000_000_000.0);
    println!("Initial Receiver {} balance: {} SOL", receiver.pubkey(), receiver_balance as f64 / 1_000_000_000.0);

    // Initialize delegation service
    println!("\nInitializing delegation service...");
    let client = reqwest::Client::new();
    let response = client
        .post("http://127.0.0.1:8080/init_delegation_service")
        .body(sender.to_bytes().to_vec())
        .send()
        .await?;
    
    println!("Delegation service init response: {:?}", response.text().await?);
    
    // Wait for initialization
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Send 10 transactions
    for i in 0..2 {
        println!("\nSending transaction {} of 2", i + 1);
        
        let test_tx = Transaction::new_signed_with_payer(
            &[system_instruction::transfer(
                &sender.pubkey(),
                &receiver.pubkey(),
                250_000_000, // 0.25 SOL
            )],
            Some(&sender.pubkey()),
            &[&sender],
            rpc_client.get_latest_blockhash().await?,
        );

        let rtx = RollupTransaction {
            sender: sender.pubkey().to_string(),
            sol_transaction: test_tx,
            keypair_bytes: sender.to_bytes().to_vec(),
        };

        let response = client
            .post("http://127.0.0.1:8080/submit_transaction")
            .json(&rtx)
            .send()
            .await?;

        println!("Transaction {} response: {:?}", i + 1, response.text().await?);
        
        // Small delay between transactions
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    // Print final balances
    let sender_balance = rpc_client.get_balance(&sender.pubkey()).await?;
    let receiver_balance = rpc_client.get_balance(&receiver.pubkey()).await?;
    println!("\nFinal Sender {} balance: {} SOL", sender.pubkey(), sender_balance as f64 / 1_000_000_000.0);
    println!("Final Receiver {} balance: {} SOL", receiver.pubkey(), receiver_balance as f64 / 1_000_000_000.0);
    
    Ok(())
}

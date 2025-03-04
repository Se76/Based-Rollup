pub mod raydium_utilis;

use anyhow::Result;
use bincode;
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::{self, RpcClient};
use solana_sdk::{
    hash::{Hash, Hasher}, native_token::LAMPORTS_PER_SOL, pubkey::Pubkey, signature::Signature, signer::{self, Signer}, system_instruction, system_program, transaction::Transaction, instruction::{AccountMeta, Instruction, CompiledInstruction, InstructionError}
};
use solana_transaction_status::UiTransactionEncoding::{self, Binary};
use core::hash;
use std::{collections::HashMap, f32::MIN, ops::Div, str::FromStr};
use spl_token;
// use serde_json;
use spl_associated_token_account;

#[derive(Serialize, Deserialize, Debug)]
struct RollupTransaction {
    sender: String,
    sol_transaction: Transaction,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetTransaction {
    pub get_tx: String,
}

const NATIVE_MINT: Pubkey = spl_token::native_mint::id();

#[tokio::main]
async fn main() -> Result<()> {
    let MINT: Pubkey = Pubkey::from_str("9djQYHX62Fz5ZBuD1FzxH3VA7WsPVqLJ8b6hgH2HSLCq").unwrap();
    let path = "/home/izomana/adv-svm/Basic_Rollup_fork/rollup_client/mykey_1.json";
    let path2 = "/home/izomana/adv-svm/Basic_Rollup_fork/rollup_client/testkey.json";
    let path3 = "/home/izomana/adv-svm/Basic_Rollup_fork/rollup_client/owner.json";
    let keypair = signer::keypair::read_keypair_file(path.to_string()).unwrap();
    let keypair2 = signer::keypair::read_keypair_file(path2.to_string()).unwrap();
    let keypair3 = signer::keypair::read_keypair_file(path3.to_string()).unwrap();
    let rpc_client = RpcClient::new("https://api.devnet.solana.com".into());

    // let tokenProgramData = rpc_client.get_account_data(&spl_token::ID).await?;
    // let tokenProgram = rpc_client.get_account(&spl_token::ID).await?;

    // println!("token program data: {:#?}", tokenProgramData);
    // println!("token program: {:#?}", tokenProgram);

    // let ix =
    //     system_instruction::transfer(&keypair2.pubkey(), &keypair.pubkey(), 1 * (LAMPORTS_PER_SOL/4));
    // let tx = Transaction::new_signed_with_payer(
    //     &[ix],
    //     Some(&keypair2.pubkey()),
    //     &[&keypair2],
    //     rpc_client.get_latest_blockhash().await.unwrap(),
    // );
    // let ix_raydium = Instruction::new_with_bytes(program_id, data, accounts)

    let ix_mint = spl_token::instruction::initialize_mint(
        &spl_token::id(),
        &Pubkey::from_str("EmXq3Ni9gfudTiyNKzzYvpnQqnJEMRw2ttnVXoJXjLo1").unwrap(),
        &keypair.pubkey(),
        None,
        6,
    ).unwrap();

    let tx_mint = Transaction::new_signed_with_payer(
        &[ix_mint],
        Some(&keypair.pubkey()),
        &[&keypair],
        rpc_client.get_latest_blockhash().await.unwrap(),
    );

    let ata_1 = spl_associated_token_account::get_associated_token_address(&keypair.pubkey(), &NATIVE_MINT);
    let ata_2 = spl_associated_token_account::get_associated_token_address(&keypair2.pubkey(), &NATIVE_MINT);

    let ix_other = spl_token_2022::instruction::close_account
    (
        &spl_token_2022::id(), 
        &ata_1, 
        &ata_2, 
        &keypair.pubkey(), 
        &[&keypair.pubkey()]
    ).unwrap();

    let tx_other = Transaction::new_signed_with_payer(
        &[ix_other],
        Some(&keypair.pubkey()),
        &[&keypair],
        rpc_client.get_latest_blockhash().await.unwrap(),
    );

    let ix2 =
        spl_token::instruction::transfer
        (
            &spl_token::id(),
            &ata_1,
            // &NATIVE_MINT,
            &ata_2,
            // &keypair.pubkey(),
            &keypair.pubkey(), 
            &[&keypair.pubkey()],
            LAMPORTS_PER_SOL/5,
            // 9,
        ).unwrap();

    let ix2_token_2022 =
        spl_token_2022::instruction::transfer_checked
        (
            &spl_token::id(),
            &ata_1,
            &NATIVE_MINT,
            &ata_2,
            &keypair.pubkey(), 
            &[&keypair.pubkey()],
            LAMPORTS_PER_SOL/4,
            9,
        ).unwrap();

    let ix3 = spl_token::instruction::transfer(
        &spl_token::id(),
        &ata_1,
        &ata_2,
        &keypair.pubkey(),
        &[],
        LAMPORTS_PER_SOL
    ).unwrap();

    let tx3 = Transaction::new_with_payer(
        &[ix3],
        Some(&keypair.pubkey()),
    );


    let tx2 = Transaction::new_signed_with_payer(
        &[ix2],
        Some(&keypair.pubkey()),
        &[&keypair],
        rpc_client.get_latest_blockhash().await.unwrap(),
    );
    let tx2_token_2022 = Transaction::new_signed_with_payer(
        &[ix2_token_2022],
        Some(&keypair.pubkey()),
        &[&keypair],
        rpc_client.get_latest_blockhash().await.unwrap(),
    );


    // println!("our tx: {:?}", tx2);
    // let sig = Signature::from_str("3ENa2e9TG6stDNkUZkRcC2Gf5saNMUFhpptQiNg56nGJ9eRBgSJpZBi7WLP5ev7aggG1JAXQWzBk8Xfkjcx1YCM2").unwrap();
    // let tx = rpc_client.get_transaction(&sig, UiTransactionEncoding::Binary).await.unwrap();


    // let signiture = rpc_client.send_and_confirm_transaction(&tx2).await;
    // println!("signitue: {}", signiture.unwrap());
    // 3SfME36kENVFcK6Z1kp4zMwCGUFDT2inSvHBcQ7gAMv7xgvNLvdNCQbpcnAYdQuwopXtti1jZpDtrwdL4XNkJWx6
    // 5kqG4z8PDwbHwEEGs4v2R9Ftnkv83gRzdZe8YHf58JTCyrr4U15NAXd8yf5DMEwg4uAvL8jLPSPTxdss16mpYXB5
    




    let client = reqwest::Client::new();

    // let tx_encoded: Transaction = tx.try_into().unwrap();

    let test_response = client
        .get("http://127.0.0.1:8080")
        .send()
        .await?
        .json::<HashMap<String, String>>()
        .await?;

    println!("{test_response:#?}");

    let rtx = RollupTransaction {
        sender: "Me".into(),
        sol_transaction: tx2_token_2022,
    };
    let submit_transaction = client
    .post("http://127.0.0.1:8080/submit_transaction")
    .json(&rtx)
    .send()
    .await?;
    // .json()
    // .await?;

    println!("{submit_transaction:#?}");







    let latest_blockhash = rpc_client.get_latest_blockhash().await.unwrap();
    for _ in 0..9{
        let loop_ix = gen_token_transfer_tx(path.into(), keypair2.pubkey().to_string(), NATIVE_MINT.to_string(), 1000000, latest_blockhash).await;
        let rtx = RollupTransaction {
            sender: "Me".into(),
            sol_transaction: loop_ix
        };
        let submit_transaction = client
        .post("http://127.0.0.1:8080/submit_transaction")
        .json(&rtx)
        .send()
        .await?;
        // .json()
        // .await?;

        println!("{submit_transaction:#?}");
    }

    // let serialized_rollup_transaction = serde_json::to_string(&rtx)?;

    //UNCOMMENT
    // let submit_transaction = client
    //     .post("http://127.0.0.1:8080/submit_transaction")
    //     .json(&rtx)
    //     .send()
    //     .await?;
    // // .json()
    // // .await?;

    // println!("{submit_transaction:#?}");
    // let mut hasher = Hasher::default();
    // hasher.hash(bincode::serialize(&rtx.sol_transaction).unwrap().as_slice());

    // println!("{:#?}", hasher.clone().result());

    // let tx_resp = client
    //     .post("http://127.0.0.1:8080/get_transaction")
    //     .json(&GetTransaction{get_tx: rtx.sol_transaction.message.hash().to_string()})
    //     .send()
    //     .await?;
    //     // .json::<HashMap<String, String>>()
    //     // .await?;

    // println!("{tx_resp:#?}");

    // let amounts: Vec<i32> = vec![4, -2, 3, -5, 1, -4, 2, -1, 3, -1];


    // UNCOMMENT


    // let amounts: Vec<(String, String, i32)> = vec![
    //     (path.to_string(), path2.to_string(), 5),
    //     (path3.to_string(), path.to_string(), -3),
    //     (path2.to_string(), path3.to_string(), 8),
    //     (path.to_string(), path3.to_string(), -7),
    //     (path2.to_string(), path.to_string(), 4),
    //     (path3.to_string(), path2.to_string(), -6),
    //     (path.to_string(), path2.to_string(), 9),
    //     (path2.to_string(), path3.to_string(), -2),
    //     (path3.to_string(), path.to_string(), 1),
    //     (path.to_string(), path3.to_string(), -4),
    // ];
    // let mut txs: Vec<Transaction> = vec![];
    // for amt in amounts {
    //     if amt.2 > 0 {
    //         txs.push(gen_transfer_tx(amt.0, amt.1, amt.2 as u64).await);
    //     } else {
    //         txs.push(gen_transfer_tx(amt.1, amt.0, amt.2.abs() as u64).await);
    //     }
    // }

    // for tx in txs {
    //     let rtx = RollupTransaction {
    //         sender: "Me".into(),
    //         sol_transaction: tx
    //     };

    //     let submission = client
    //         .post("http://127.0.0.1:8080/submit_transaction")
    //         .json(&rtx)
    //         .send()
    //         .await?;
        
    //     println!("Submission {submission:#?}");
    // }

    // println!("KP: {}", keypair.pubkey());
    // println!("KP2: {}", keypair2.pubkey());

    Ok(())
}

async fn gen_transfer_tx(path1: String, path2: String, amount: u64) -> Transaction {
    println!("Amount: {amount}");
    let keypair = signer::keypair::read_keypair_file(path1.to_string()).unwrap();
    let keypair2 = signer::keypair::read_keypair_file(path2.to_string()).unwrap();
    let rpc_client = RpcClient::new("https://api.devnet.solana.com".into());

    let ix =
        system_instruction::transfer(&keypair2.pubkey(), &keypair.pubkey(), amount * (LAMPORTS_PER_SOL / 10));
    Transaction::new_signed_with_payer(
        &[ix],
        Some(&keypair2.pubkey()),
        &[&keypair2],
        rpc_client.get_latest_blockhash().await.unwrap(),
    )
}

async fn gen_token_transfer_tx(
    sender_key_path: String,
    recipient_pubkey: String,
    mint_pubkey: String,
    amount: u64,
    latest_blockhash: Hash
) -> Transaction {
    println!("Amount: {amount}");

    let sender_keypair = signer::keypair::read_keypair_file(sender_key_path).unwrap();
    let recipient_pubkey = recipient_pubkey.parse::<Pubkey>().unwrap();
    let mint_pubkey = mint_pubkey.parse::<Pubkey>().unwrap();

    let rpc_client = RpcClient::new("https://api.devnet.solana.com".into());

    // Get associated token accounts
    let sender_token_account = spl_associated_token_account::get_associated_token_address(
        &sender_keypair.pubkey(),
        &mint_pubkey
    );
    let recipient_token_account = spl_associated_token_account::get_associated_token_address(
        &recipient_pubkey,
        &mint_pubkey
    );

    // Create the transfer instruction
    let transfer_ix = spl_token::instruction::transfer(
        &spl_token::ID,
        &sender_token_account,
        &recipient_token_account,
        &sender_keypair.pubkey(),
        &[],
        amount,
    ).unwrap();

    // Create and sign the transaction
    Transaction::new_signed_with_payer(
        &[transfer_ix],
        Some(&sender_keypair.pubkey()),
        &[&sender_keypair],
        latest_blockhash,
    )
}
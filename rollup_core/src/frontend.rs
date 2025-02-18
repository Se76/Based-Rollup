use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use actix_web::{error, web, HttpResponse};
use async_channel::{Receiver, Send, Sender};
use crossbeam::channel::{Sender as CBSender, Receiver as CBReceiver};
use serde::{Deserialize, Serialize};
use solana_sdk::hash::Hash; // keccak::Hash
use solana_sdk::transaction::Transaction;
use {
    crate::delegation_service::DelegationService,
    solana_sdk::pubkey::Pubkey,
    std::sync::{RwLock},
};

use crate::rollupdb::RollupDBMessage;

// message format to send found transaction from db to frontend
#[derive(Serialize, Deserialize)]
pub struct FrontendMessage {
    pub get_tx: Option<Hash>,
    pub transaction: Option<Transaction>,
}

// message format used to get transaction client
#[derive(Serialize, Deserialize, Debug)]
pub struct GetTransaction {
    pub get_tx: String,
}

// message format used to receive transactions from clients
#[derive(Serialize, Deserialize, Debug)]
pub struct RollupTransaction {
    sender: String,
    sol_transaction: Transaction,
}

#[derive(Serialize)]
pub enum TransactionResponse {
    Success { message: String },
    NeedsDelegation { delegation_tx: Transaction },
    Error { message: String },
}

pub async fn submit_transaction(
    body: web::Json<RollupTransaction>,
    sequencer_sender: web::Data<CBSender<Transaction>>,
    delegation_service: web::Data<Arc<RwLock<DelegationService>>>,
) -> actix_web::Result<HttpResponse> {
    let tx = &body.sol_transaction;
    let payer = tx.message.account_keys[0];
    let amount = 1_000_000; // Extract actual amount

    // Check delegation and get PDA
    let delegation_result = delegation_service.write().unwrap()
        .verify_delegation_for_transaction(&payer, amount)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    match delegation_result {
        Some(pda) => {
            // Modify transaction to use PDA
            let mut modified_tx = tx.clone();
            modified_tx.message.account_keys[0] = pda;

            // Send modified transaction to sequencer
            sequencer_sender.send(modified_tx)
                .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

            Ok(HttpResponse::Ok().json(TransactionResponse::Success {
                message: "Transaction submitted".to_string()
            }))
        }
        None => {
            // Create delegation transaction
            let delegation_tx = delegation_service.write().unwrap()
                .create_delegation_transaction(&payer, amount)
                .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

            Ok(HttpResponse::Ok().json(TransactionResponse::NeedsDelegation {
                delegation_tx
            }))
        }
    }
}

pub async fn get_transaction(
    body: web::Json<GetTransaction>,
    sequencer_sender: web::Data<Sender<Transaction>>,
    rollupdb_sender: web::Data<Sender<RollupDBMessage>>,
    frontend_receiver: web::Data<Receiver<FrontendMessage>>,
) -> actix_web::Result<HttpResponse> {
    // Validate transaction structure with serialization in function signature
    log::info!("Requested transaction");
    log::info!("{body:?}");

    rollupdb_sender
        .send(RollupDBMessage {
            lock_accounts: None,
            add_new_data: None,
            add_processed_transaction: None,
            frontend_get_tx: Some(Hash::new(body.get_tx.as_bytes())),
            add_settle_proof: None,
            get_account: None,
        })
        .await
        .unwrap();

    if let Ok(frontend_message) = frontend_receiver.recv().await {
        return Ok(HttpResponse::Ok().json(RollupTransaction {
            sender: "Rollup RPC".into(),
            sol_transaction: frontend_message.transaction.unwrap(),
        }));
        // Ok(HttpResponse::Ok().json(HashMap::from([("Transaction status", "requested")])))
    }

    Ok(HttpResponse::Ok().json(HashMap::from([("Transaction status", "requested")])))
}

pub async fn test() -> HttpResponse {
    log::info!("Test request");
    HttpResponse::Ok().json(HashMap::from([("test", "success")]))
}

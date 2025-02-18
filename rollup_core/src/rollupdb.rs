use async_channel::{Receiver, Sender};
use log::log;
use serde::{Deserialize, Serialize};
use solana_sdk::{
    account::AccountSharedData, hash::Hash, pubkey::Pubkey, transaction::Transaction, // keccak::Hash -> hash::Hash
};

use crossbeam::channel::{Receiver as CBReceiver, Sender as CBSender};
use std::{
    collections::{HashMap, HashSet},
    default,
};

use crate::frontend::FrontendMessage;
use crate::bundler::*;

#[derive(Serialize, Deserialize)]
pub struct RollupDBMessage {
    pub lock_accounts: Option<Vec<Pubkey>>,
    pub add_processed_transaction: Option<Transaction>,
    pub frontend_get_tx: Option<Hash>,
    pub add_settle_proof: Option<String>,
    //Testing purposes
    pub bundle_tx: bool
}

#[derive(Serialize, Debug, Default)]
pub struct RollupDB {
    accounts_db: HashMap<Pubkey, AccountSharedData>,
    locked_accounts: HashMap<Pubkey, AccountSharedData>,
    transactions: HashMap<Hash, Transaction>,
}

impl RollupDB {
    pub async fn run(
        rollup_db_receiver: CBReceiver<RollupDBMessage>,
        frontend_sender: Sender<FrontendMessage>,
    ) {
        let mut db = RollupDB {
            accounts_db: HashMap::new(),
            locked_accounts: HashMap::new(),
            transactions: HashMap::new(),
        };

        while let Ok(message) = rollup_db_receiver.recv() {
            log::info!("Received RollupDBMessage");
            if let Some(accounts_to_lock) = message.lock_accounts {
                // Lock accounts, by removing them from the accounts_db hashmap, and adding them to locked accounts
                let _ = accounts_to_lock.iter().map(|pubkey| {
                    db.locked_accounts
                        .insert(pubkey.clone(), db.accounts_db.remove(pubkey).unwrap())
                });
            } else if let Some(get_this_hash_tx) = message.frontend_get_tx {
                log::info!("Getting tx for frontend");
                let req_tx = db.transactions.get(&get_this_hash_tx).unwrap();

                frontend_sender
                    .send(FrontendMessage {
                        transaction: Some(req_tx.clone()),
                        get_tx: None,
                    })
                    .await
                    .unwrap();
            } else if let Some(tx) = message.add_processed_transaction {
                log::info!("Adding processed tx");
                // unlocking accounts
                let locked_keys = tx.message.account_keys.clone(); // get the keys

                // locked_keys.iter().for_each(
                //     |pubkey| if db.locked_accounts.contains_key(&pubkey) {
                //         db.locked_accounts.remove(&pubkey);
                //     }
                // );

                for pubkey in locked_keys {
                    if let Some(account) = db.locked_accounts.remove(&pubkey) {
                        db.accounts_db.insert(pubkey, account); // Unlock and restore
                    }
                }
                // send transaction to the db.transactions

                db.transactions.insert(tx.message.hash(), tx.clone());
                log::info!("PROCESSED TX: {}", db.transactions.len());

                // communication channel with database 
                // communcation with the frontend 
            } else if message.bundle_tx {
                log::info!("BUNDLING TX");
                let mut tx_bundler = TransferBundler::new();
                for (_, tx) in db.transactions.clone() {
                    tx_bundler.bundle(tx);
                }
                let final_ixs = tx_bundler.generate_final();
                log::info!("\nFinal Transfer Ixs:");
                for ix in final_ixs{
                    if let Some((from, to, amount)) = TransferBundler::parse_instruction(&ix){
                    }
                }
                log::info!("BUNDLING DONE");
                db.transactions.clear();
            }
        }
    }
}

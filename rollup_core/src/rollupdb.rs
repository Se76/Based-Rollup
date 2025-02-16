use async_channel::{Receiver, Sender};
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    account::AccountSharedData, hash::Hash, pubkey::Pubkey, transaction::Transaction, // keccak::Hash -> hash::Hash
};

use crossbeam::channel::{Receiver as CBReceiver, Sender as CBSender};
use std::{
    collections::{HashMap, HashSet},
    default,
};
use tokio::sync::oneshot;
use crate::frontend::FrontendMessage;

#[derive(Serialize, Deserialize)]
pub struct RollupDBMessage {
    pub lock_accounts: Option<Vec<Pubkey>>,
    pub add_processed_transaction: Option<Transaction>,
    pub add_new_data: Option<Vec<(Pubkey, AccountSharedData)>>,
    pub frontend_get_tx: Option<Hash>,
    pub add_settle_proof: Option<String>,
    pub get_account: Option<Pubkey>,
    // pub response: Option<bool>, 
}

#[derive(Serialize, Debug, Default)]
pub struct RollupDB {
    accounts_db: HashMap<Pubkey, AccountSharedData>,
    locked_accounts: HashMap<Pubkey, AccountSharedData>,
    transactions: HashMap<Hash, Transaction>,
    // async_ver_recv: Receiver<Option<bool>>
}

impl RollupDB {
    pub async fn run(
        rollup_db_receiver: CBReceiver<RollupDBMessage>,
        frontend_sender: Sender<FrontendMessage>,
        account_sender: Sender<Option<Vec<(Pubkey, AccountSharedData)>>>
    ) {
        let mut db = RollupDB {
            accounts_db: HashMap::new(),
            locked_accounts: HashMap::new(),
            transactions: HashMap::new(),
        };
        while let Ok(message) = rollup_db_receiver.recv() {
            if let Some(accounts_to_lock) = message.lock_accounts {
                let mut information_to_send: Vec<(Pubkey, AccountSharedData)> = Vec::new();
                log::info!("locking: {:?}", db.accounts_db);
                // Lock accounts, by removing them from the accounts_db hashmap, and adding them to locked accounts
                for pubkey in accounts_to_lock.iter() {
                    if let Some(account) = db.accounts_db.get(pubkey) {
                        db.locked_accounts
                        .insert(pubkey.clone(), db.accounts_db.remove(pubkey).unwrap());
                        log::info!("account was found");
                    }
                    else {
                        let rpc_client_temp = RpcClient::new("https://api.devnet.solana.com".to_string());
                        let account = rpc_client_temp.get_account(pubkey).unwrap();
                        let data: AccountSharedData = account.into();
                        db.locked_accounts
                        .insert(pubkey.clone(), data);
                        log::info!("account was not found");
                    }

                    if let Some(account) = db.locked_accounts.get(&pubkey) {
                        // account_sender.send(Some(account.clone())).await.unwrap();
                        information_to_send.push((*pubkey, account.clone()));
                    }
                    else {
                        // account_sender.send(None).await.unwrap();
                        panic!()
                    }

                }
                log::info!("locking done: {:?}", db.accounts_db);
                log::info!("locked accounts done: {:?}", db.locked_accounts);

                
                log::info!("information to send -> {:?}", information_to_send);
                account_sender.send(Some(information_to_send)).await.unwrap();
                // log::info!("2: {:#?}", db.locked_accounts);
            } else if let Some(get_this_hash_tx) = message.frontend_get_tx {
                let req_tx = db.transactions.get(&get_this_hash_tx).unwrap();

                frontend_sender
                    .send(FrontendMessage {
                        transaction: Some(req_tx.clone()),
                        get_tx: None,
                    })
                    .await
                    .unwrap();
            } else if let Some(tx) = message.add_processed_transaction {

                let processed_data = message.add_new_data.unwrap();

                // unlocking accounts
                let locked_keys = tx.message.account_keys.clone(); // get the keys
                log::info!("it is starting accounts_db{:#?}", db.accounts_db);
                log::info!("it is starting locked_db{:#?}", db.locked_accounts);
                for (pubkey, data) in processed_data.iter() {
                    db.locked_accounts.remove(pubkey).unwrap();
                    db.accounts_db.insert(*pubkey, data.clone());
                    log::info!("it is final accounts_db{:#?}", db.accounts_db);
                    log::info!("it is final locked_db{:#?}", db.locked_accounts);
                    

                }
                // send transaction to the db.transactions

                db.transactions.insert(tx.message.hash(), tx.clone());
                log::info!("locked: {:#?}", db.locked_accounts);
                log::info!("43210: {:#?}", db.accounts_db);

                // communication channel with database 
                // communcation with the frontend 
            }
            // else if let Some(pubkey) = message.get_account {
                
            //     log::info!("4321: {:#?}", db.locked_accounts);
            //     if let Some(account) = db.locked_accounts.get(&pubkey) {
            //         account_sender.send(Some(account.clone())).await.unwrap();
            //     }
            //     else {
            //         account_sender.send(None).await.unwrap();
            //     }
            // }
        }
    }
}



                    // if let Some(account) = db.locked_accounts.remove(&pubkey) {
                    //     let data_for_the_account = data.get(pubkey).unwrap()
                    //     db.accounts_db.insert(pubkey, data.get(pubkey).unwrap()); // Unlock and restore
                    // }
// accounts_to_lock
                // .iter()
                // .map(|pubkey| {
                //     // match db.locked_accounts.get(pubkey) {
                //     //     Some(account) => {
                //     //         db.locked_accounts
                //     //     .insert(pubkey.clone(), db.accounts_db.remove(pubkey).unwrap());
                //     //     }
                //     //     None => {
                //     //     let rpc_client_temp = RpcClient::new("https://api.devnet.solana.com".to_string());
                //     //     let account = rpc_client_temp.get_account(pubkey).unwrap();
                //     //     let data: AccountSharedData = account.into();
                //     //     db.locked_accounts
                //     //     .insert(pubkey.clone(), data);
                //     //     }
                //     // }
                //     log::info!("99999999999999999999999999999999");
                //     if let Some(account) = db.accounts_db.get(pubkey) {
                //         db.locked_accounts
                //         .insert(pubkey.clone(), db.accounts_db.remove(pubkey).unwrap());
                //         log::info!("111111111111111111111112");
                //     }
                //     else {
                //         let rpc_client_temp = RpcClient::new("https://api.devnet.solana.com".to_string());
                //         let account = rpc_client_temp.get_account(pubkey).unwrap();
                //         let data: AccountSharedData = account.into();
                //         db.locked_accounts
                //         .insert(pubkey.clone(), data);
                //     log::info!("2222222222222222222222222");
                //     }
                // });


                // let _ = accounts_to_lock.iter().map(|pubkey| {
                //     db.locked_accounts
                //         .insert(pubkey.clone(), db.accounts_db.remove(pubkey).unwrap())
                // });

use core::panic;
use std::{
    collections::{HashMap, HashSet}, sync::{Arc, RwLock}, time, vec
};

use anyhow::{anyhow, Result};
use async_channel::{Sender, Receiver};
use crossbeam::channel::{Sender as CBSender, Receiver as CBReceiver};
use solana_client::{nonblocking::rpc_client as nonblocking_rpc_client, rpc_client::RpcClient};
use solana_compute_budget::compute_budget::ComputeBudget;
use solana_program_runtime::{
    invoke_context::{self, EnvironmentConfig, InvokeContext},
    loaded_programs::{BlockRelation, ForkGraph, LoadProgramMetrics, ProgramCacheEntry, ProgramCacheForTxBatch, ProgramRuntimeEnvironments}, sysvar_cache,
};

use solana_bpf_loader_program::syscalls::create_program_runtime_environment_v1;
use solana_sdk::{
    account::{AccountSharedData, ReadableAccount}, clock::{Epoch, Slot}, feature_set::FeatureSet, fee::FeeStructure, hash::Hash, instruction, pubkey::Pubkey, rent::Rent, rent_collector::RentCollector, sysvar::instructions, transaction::{SanitizedTransaction, Transaction}, transaction_context::{IndexOfAccount, TransactionContext},
};
use solana_timings::ExecuteTimings;
use solana_svm::{
transaction_processing_callback::TransactionProcessingCallback, transaction_processing_result::ProcessedTransaction, transaction_processor::{TransactionBatchProcessor, TransactionProcessingConfig, TransactionProcessingEnvironment}
};
use tokio::time::{sleep, Duration};
use crate::{rollupdb::RollupDBMessage, settle::settle_state};
use crate::loader::RollupAccountLoader;
use crate::processor::*;
use crate::errors::RollupErrors;
use crate::delegation_service::DelegationService;

pub async fn run(
    sequencer_receiver_channel: CBReceiver<Transaction>,
    rollupdb_sender: CBSender<RollupDBMessage>,
    account_reciever: Receiver<Option<Vec<(Pubkey, AccountSharedData)>>>,
    receiver_locked_accounts: Receiver<bool>,
) -> Result<()> {
    let mut tx_counter = 0u32;

    let rpc_client_temp = RpcClient::new("https://api.devnet.solana.com".to_string());

    let mut rollup_account_loader = RollupAccountLoader::new(
        &rpc_client_temp,
    );
    while let transaction = sequencer_receiver_channel.recv().unwrap() {
        let accounts_to_lock = transaction.message.account_keys.clone();
        for pubkey in accounts_to_lock.iter() {
            loop {
                rollupdb_sender
                .send(RollupDBMessage {
                    lock_accounts: None,
                    frontend_get_tx: None,
                    add_settle_proof: None,
                    add_new_data: None,
                    add_processed_transaction: None,
                    get_account: Some(*pubkey),
            })
            
            .map_err(|_| anyhow!("failed to send message to rollupdb"))?;
                if receiver_locked_accounts.recv().await.unwrap() == false {
                    break;
                }
                sleep(Duration::from_millis(500)).await;
            }
        }
        tx_counter += 1;
        // lock accounts in rollupdb to keep paralell execution possible, just like on solana
        rollupdb_sender
            .send(RollupDBMessage {
                lock_accounts: Some(accounts_to_lock),
                frontend_get_tx: None,
                add_settle_proof: None,
                add_new_data: None,
                add_processed_transaction: None,
                get_account: None,
                // response: Some(true), 
            })
            
            .map_err(|_| anyhow!("failed to send message to rollupdb"))?;

        if let Some(vec_of_accounts_data) = account_reciever.recv().await.unwrap() {
            log::info!("received::: {:?}", vec_of_accounts_data);
            for (pubkey, account) in vec_of_accounts_data.iter() {
                rollup_account_loader.add_account(*pubkey, account.clone());
                log::info!("sucess:")
            }
        }
        for pubkey in transaction.message.account_keys.iter(){
            let data = rollup_account_loader.get_account_shared_data(pubkey);
            log::info!("data from an account: {:?}", data);
        }

        let compute_budget = ComputeBudget::default();
        let feature_set = FeatureSet::all_enabled();
        let mut fee_structure = FeeStructure::default();
        fee_structure.lamports_per_signature = 0;
        let lamports_per_signature = fee_structure.lamports_per_signature;
        let rent_collector = RentCollector::default();
        let mut timings = ExecuteTimings::default();
        let fork_graph = Arc::new(RwLock::new(RollupForkGraph {}));

        let mut used_cu = 0u64;
        let sanitized = SanitizedTransaction::try_from_legacy_transaction( // to check here for the problem
            Transaction::from(transaction.clone()),
            &HashSet::new(),
        );

        log::info!("{:?}", sanitized.clone());

        let payer = transaction.message.account_keys[0];
        let amount = 1_000_000; // For now, using a fixed amount. Replace with actual amount extraction

        let processor = create_transaction_batch_processor(
            &rollup_account_loader,
            &feature_set,
            &compute_budget,
            Arc::clone(&fork_graph),
        );

        let checks = get_transaction_check_results(1, fee_structure.lamports_per_signature);
        let sanitized_transaction = &[sanitized.unwrap()]; 

        let processing_environment = TransactionProcessingEnvironment {
            blockhash: Hash::default(),
            epoch_total_stake: 0u64,
            feature_set: Arc::new(feature_set),
            rent_collector: Some(&rent_collector),
            blockhash_lamports_per_signature: fee_structure.lamports_per_signature,
            fee_lamports_per_signature: fee_structure.lamports_per_signature,
        };

        let processing_config = TransactionProcessingConfig {
            compute_budget: Some(compute_budget),
            ..Default::default()
        };

        let status = processor.load_and_execute_sanitized_transactions(
            &rollup_account_loader, 
            sanitized_transaction, 
            checks, 
            &processing_environment, 
            &processing_config
        );
        log::info!("{:#?}", status.processing_results);
        log::info!("error_metrics: {:#?}", status.error_metrics);

        let data_new = 
        status
        .processing_results
        .iter()
        .map(|res| {
            println!("Executed transaction:");
            log::info!("Executed transaction");
            let enum_one = res.as_ref().unwrap();
    
            match enum_one {
                ProcessedTransaction::Executed(tx) => {
                    println!("Executed transaction: {:?}", tx.loaded_transaction.accounts);
                    Some(tx.loaded_transaction.accounts.clone()) 
                }
                ProcessedTransaction::FeesOnly(tx) => {
                    println!("Fees-only transaction: {:?}", tx);
                    None 
                }
            }
        }).collect::<Vec<Option<Vec<(Pubkey, AccountSharedData)>>>>();

        let first_index_data = data_new[0].as_ref().unwrap().clone();
        log::info!("swq {:?}", first_index_data);
             // Send processed transaction to db for storage and availability
        rollupdb_sender
            .send(RollupDBMessage {
                lock_accounts: None,
                add_processed_transaction: Some(transaction),
                add_new_data: Some(first_index_data),
                frontend_get_tx: None,
                add_settle_proof: None,
                get_account: None,
            })
            
            .unwrap();

        // Call settle if transaction amount since last settle hits 10
        if tx_counter >= 10 {
            tx_counter = 0u32;
        }
    }
    Ok(())
}

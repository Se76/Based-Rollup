use core::panic;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
};

use anyhow::{anyhow, Result};
use async_channel::Sender;
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
    message_processor::MessageProcessor,
    program_loader::load_program_with_pubkey,
    transaction_processing_callback::TransactionProcessingCallback,
    transaction_processor::{TransactionBatchProcessor, TransactionProcessingConfig, TransactionProcessingEnvironment},
};

use crate::{rollupdb::RollupDBMessage, settle::settle_state};
use crate::loader::RollupAccountLoader;
use crate::processor::*;
use crate::bundler::*;

pub fn run(
    sequencer_receiver_channel: CBReceiver<Transaction>,
    rollupdb_sender: CBSender<RollupDBMessage>,
) -> Result<()> {
    let mut tx_counter = 0u32;
    while let transaction = sequencer_receiver_channel.recv().unwrap() {
        let accounts_to_lock = transaction.message.account_keys.clone();
        tx_counter += 1;
        // lock accounts in rollupdb to keep paralell execution possible, just like on solana
        rollupdb_sender
            .send(RollupDBMessage {
                lock_accounts: Some(accounts_to_lock),
                frontend_get_tx: None,
                add_settle_proof: None,
                add_processed_transaction: None,
                bundle_tx: false
            })
            
            .map_err(|_| anyhow!("failed to send message to rollupdb"))?;

        // Verify ransaction signatures, integrity

        // Process transaction

        let compute_budget = ComputeBudget::default();
        let feature_set = FeatureSet::all_enabled();
        let fee_structure = FeeStructure::default();
        let lamports_per_signature = fee_structure.lamports_per_signature;
        let rent_collector = RentCollector::default();
        let mut timings = ExecuteTimings::default();
        let fork_graph = Arc::new(RwLock::new(RollupForkGraph {}));


        
        // let rent_collector = RentCollector::default();

        // Solana runtime.
        // let fork_graph = Arc::new(RwLock::new(SequencerForkGraph {}));

        // // create transaction processor, add accounts and programs, builtins,
        // let processor = TransactionBatchProcessor::<SequencerForkGraph>::default();

        // let mut cache = processor.program_cache.write().unwrap();

        // // Initialize the mocked fork graph.
        // // let fork_graph = Arc::new(RwLock::new(PayTubeForkGraph {}));
        // cache.fork_graph = Some(Arc::downgrade(&fork_graph));

        // let rent = Rent::default();

        let rpc_client_temp = RpcClient::new("https://api.devnet.solana.com".to_string());

        let accounts_data = transaction // adding reference
            .message
            .account_keys
            .iter()
            .map(|pubkey| {
                (
                    pubkey.clone(),
                    rpc_client_temp.get_account(pubkey).unwrap().into(),
                )
            })
            .collect::<Vec<(Pubkey, AccountSharedData)>>();

        let mut used_cu = 0u64;
        let sanitized = SanitizedTransaction::try_from_legacy_transaction( // to check here for the problem
            Transaction::from(transaction.clone()),
            &HashSet::new(),
        );

        log::info!("Sanitized txs: {:?}", sanitized.clone());

        //TO DELETE: Detect if transaction contains a transfer ix
        let ixs = get_transaction_instructions(&transaction);
        let acc_keys: &[Pubkey] = &transaction.message.account_keys;
        ixs.iter().for_each(|ix| {
            if is_transfer_ix(ix, acc_keys){
                log::info!("\n\nTransfer IX detected: \n{:?}\n\n", ix);
            } else {
                log::info!("\nIX not a transfer IX\n");
            }
        });

        let needed_programs: Vec<(Pubkey, AccountSharedData)> = 
        accounts_data
        .iter()
        .filter(|(pubkey, account)| account.executable())
        .map(|(pubkey, account)| (pubkey.clone(), account.clone()))
        .collect();

        log::info!("accounts_data: {needed_programs:?}");

        let mut rollup_account_loader = RollupAccountLoader::new(
            &rpc_client_temp,
        );

        for (pubkey, account) in needed_programs.iter() {
            rollup_account_loader.add_account(*pubkey, account.clone());
        }


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
            epoch_total_stake: None,
            epoch_vote_accounts: None,
            feature_set: Arc::new(feature_set),
            fee_structure: Some(&fee_structure),
            lamports_per_signature: fee_structure.lamports_per_signature,
            rent_collector: Some(&rent_collector),
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
        log::info!("Processing results: {:#?}", status.processing_results);

        //TO DELETE: Check to confirm if transfer instructions are correctly parsed
        let ixs = get_transaction_instructions(&transaction);
        let acc_keys: &[Pubkey] = &transaction.message.account_keys;
        let (from, to, amount) = TransferBundler::parse_compiled_instruction(&ixs[0], acc_keys).unwrap();
        log::info!("\nFrom: {from:?}\nTo: {to:?}\nAmount: {amount}\n");
        
             // Send processed transaction to db for storage and availability
        rollupdb_sender
            .send(RollupDBMessage {
                lock_accounts: None,
                add_processed_transaction: Some(transaction.clone()),
                frontend_get_tx: None,
                add_settle_proof: None,
                bundle_tx: false
            })
            
            .unwrap();

        //View sent processed tx details
        let ixs = get_transaction_instructions(&transaction);
        let acc_keys: &[Pubkey] = &transaction.message.account_keys;
        if let Some((from, to, amount)) = TransferBundler::parse_compiled_instruction(&ixs[0], acc_keys) {
                log::info!("
                    Transaction Info\n
                    From: {from:?}\n
                    To: {to:?}\n
                    Amount: {amount}

                ")
            }

        // Call settle if transaction amount since last settle hits 10
        if tx_counter >= 10 {
            //bundle transfer tx test
            rollupdb_sender.send(RollupDBMessage {
                lock_accounts: None,
                add_processed_transaction: None,
                add_settle_proof: None,
                frontend_get_tx: None,
                bundle_tx: true
            }).unwrap();

            // Lock db to avoid state changes during settlement

            // Prepare root hash, or your own proof to send to chain

            // Send proof to chain

            // let _settle_tx_hash = settle_state("proof".into()).await?;
            tx_counter = 0u32;


            // CREATE A PROOF FOR THE CHANGES STATE
        }
    }

    Ok(())
}




 
    //         //****************************************************************************************************/
    //     // let instructions = &transaction.message.instructions; 
    //     // // let index_array_of_program_pubkeys = Vec::with_capacity(instructions.len());
    //     // let program_ids = &transaction.message.account_keys; 

    //     // let needed_programs: Vec<&Pubkey> = instructions
    //     //         .iter()
    //     //         .map(
    //     //             |instruction|
    //     //             instruction.program_id(program_ids)).collect();
    //         //****************************************************************************************************/

    //     let mut transaction_context = TransactionContext::new(
    //         accounts_data, 
    //         Rent::default(), 
    //         compute_budget.max_instruction_stack_depth,
    //     compute_budget.max_instruction_trace_length,
    // );
    //     // transaction_context.get_current_instruction_context().unwrap().get_index_of_program_account_in_transaction(2).unwrap();
    //     // transaction_context.push(); 


    //         // here we have to load them somehow

    //     let runtime_env = Arc::new(
    //         create_program_runtime_environment_v1(&feature_set, &compute_budget, false, false)
    //             .unwrap(),
    //     );

    //     let mut prog_cache = ProgramCacheForTxBatch::new(
    //         Slot::default(), 
    //         ProgramRuntimeEnvironments {
    //             program_runtime_v1: runtime_env.clone(),
    //             program_runtime_v2: runtime_env,
    //         },
    //         None, 
    //         Epoch::default(),
    //     );
        

    //     // prog_cache.replenish(accounts_data., entry)

    //     let sysvar_c = sysvar_cache::SysvarCache::default();
    //     let env = EnvironmentConfig::new(
    //         Hash::default(),
    //         None,
    //         None,
    //         Arc::new(feature_set),
    //         lamports_per_signature,
    //         &sysvar_c,
    //     );
    //     // let default_env = EnvironmentConfig::new(blockhash, epoch_total_stake, epoch_vote_accounts, feature_set, lamports_per_signature, sysvar_cache)

    //     // let processing_environment = TransactionProcessingEnvironment {
    //     //     blockhash: Hash::default(),
    //     //     epoch_total_stake: None,
    //     //     epoch_vote_accounts: None,
    //     //     feature_set: Arc::new(feature_set),
    //     //     fee_structure: Some(&fee_structure),
    //     //     lamports_per_signature,
    //     //     rent_collector: Some(&rent_collector),
    //     // };

        

    //     // for (pubkey, account) in rollup_account_loader.cache.read().unwrap().iter() {
    //     //     let _p = rollup_account_loader.get_account_shared_data(pubkey);
    //     //     log::info!("account: {_p:?}");
    //     // }
    //     // let cache = &rollup_account_loader.cache.read().unwrap();
    //     // let pew = cache.keys().next().cloned().unwrap();
    //     // let owner = cache.get(&pew).unwrap().owner();
    //     // log::debug!("pubkey: {owner:?}");
        

    //     let program_cache_entry = load_program_with_pubkey(
    //         &rollup_account_loader,
    //         &prog_cache.environments,
    //         &rollup_account_loader.cache.read().unwrap().keys().next().cloned().unwrap(),//&needed_programs[0].0,
    //         0,
    //         &mut ExecuteTimings::default(),
    //         false
    //     );

    //     log::info!("program_cache_entry: {program_cache_entry:?}");

    //     prog_cache.replenish(
    //         needed_programs[0].0,
    //         program_cache_entry.unwrap(),
    //     );
    //     // {
    //     //     let instruction_ctx = transaction_context.get_current_instruction_context();
    //     //     log::debug!("instruction_ctx: {instruction_ctx:?}");
    //     // }
    //     // let instruction_ctx_height = transaction_context.get_instruction_context_stack_height();

    //     // log::debug!("instruction_ctx_height: {instruction_ctx_height}");

    //     // let instruction_ctx_next = transaction_context.get_next_instruction_context();
    //     // // let instruction_ctx = transaction_context.get_next_instruction_context();
        
    //     // log::debug!("instruction_ctx: {instruction_ctx_next:?}");


        
    //     let mut invoke_context = InvokeContext::new(
    //        &mut transaction_context,
    //        &mut prog_cache,
    //        env,
    //        None,
    //        compute_budget.to_owned()
    //     );
        

    //     // let instruction_ctx_2 = invoke_context.transaction_context.get_current_instruction_context();
    //     // log::debug!("instruction_ctx_2: {instruction_ctx_2:?}");
    //     // let instruction_ctx_height = invoke_context.transaction_context.get_instruction_context_stack_height();
    //     // log::debug!("instruction_ctx_height: {instruction_ctx_height}");
    //     // let instruction_ctx_height = invoke_context.transaction_context.get_instruction_context_at_index_in_trace(0);
    //     // log::debug!("instruction_ctx_height: {instruction_ctx_height:?}");
        



    //     // HAS TO BE AN ADDRESS OF THE PROGRAM 

    //     // invoke_context.program_cache_for_tx_batch.replenish(key, program_cache_entry.unwrap());



        



    //     // let account_index = invoke_context
    //     //         .transaction_context
    //     //         .find_index_of_account(&instructions::id());

    //     // if account_index.is_none() {
    //     //     panic!("Could not find instructions account");
    //     // }

    //     let program_indices: Vec<IndexOfAccount> = vec![0];
    //     let result_msg = MessageProcessor::process_message(
    //         &sanitized.unwrap().message().to_owned(), // ERROR WITH SOLANA_SVM VERSION 
    //         // ?should be fixed with help of chagning versions of solana-svm ?
    //         // &sanitized.unwrap().message().to_owned(),
    //         &[program_indices],  // TODO: automotize this process
    //         &mut invoke_context,
    //         &mut timings,
    //         &mut used_cu,
    //     );

    //     log::info!("{:?}", &result_msg);
    //     log::info!("The message was done sucessfully");



   


// TWO WAYS -> TRANSACTIONBATCHPROCCESOR OR MESSAGEPROCESSOR

// PAYTUBE in SVM FOLDER

// The question of how often to pull/push the state out of mainnet state

// PDA as a *treasury , to solve problem with sol that could disapear from account 

// to create kind of a program that will lock funds on mainnet 

// MagicBlock relyaing on their infrustructure 

// To make a buffer between sending two transactions




// / In order to use the `TransactionBatchProcessor`, another trait - Solana
// / Program Runtime's `ForkGraph` - must be implemented, to tell the batch
// / processor how to work across forks.
// /
// /// Since our rollup doesn't use slots or forks, this implementation is mocked.
// pub(crate) struct SequencerForkGraph {}

// impl ForkGraph for SequencerForkGraph {
//     fn relationship(&self, _a: Slot, _b: Slot) -> BlockRelation {
//         BlockRelation::Unknown
//     }
// }
// pub struct SequencerAccountLoader<'a> {
//     cache: RwLock<HashMap<Pubkey, AccountSharedData>>,
//     rpc_client: &'a RpcClient,
// }

// impl<'a> SequencerAccountLoader<'a> {
//     pub fn new(rpc_client: &'a RpcClient) -> Self {
//         Self {
//             cache: RwLock::new(HashMap::new()),
//             rpc_client,
//         }
//     }
// }

// / Implementation of the SVM API's `TransactionProcessingCallback` interface.
// /
// / The SVM API requires this plugin be provided to provide the SVM with the
// / ability to load accounts.
// /
// / In the Agave validator, this implementation is Bank, powered by AccountsDB.
// impl TransactionProcessingCallback for SequencerAccountLoader<'_> {
//     fn get_account_shared_data(&self, pubkey: &Pubkey) -> Option<AccountSharedData> {
//         if let Some(account) = self.cache.read().unwrap().get(pubkey) {
//             return Some(account.clone());
//         }

//         let account: AccountSharedData = self.rpc_client.get_account(pubkey).ok()?.into();
//         self.cache.write().unwrap().insert(*pubkey, account.clone());

//         Some(account)
//     }

//     fn account_matches_owners(&self, account: &Pubkey, owners: &[Pubkey]) -> Option<usize> {
//         self.get_account_shared_data(account)
//             .and_then(|account| owners.iter().position(|key| account.owner().eq(key)))
//     }
// }

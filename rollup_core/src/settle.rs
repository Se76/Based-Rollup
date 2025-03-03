use anyhow::Result;
use async_channel::Receiver;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{hash::Hash, instruction::Instruction, message::Message, signature::Keypair, transaction::Transaction};

// Settle the state on solana, called by sequencer
pub async fn settle_state(vec_of_instructions: &Vec<Instruction>, signer: &Keypair) -> Result<String> {

    let rpc_client = RpcClient::new("https://api.devnet.solana.com".into());

    let ix = vec_of_instructions[0].clone();
    let payer = &ix.accounts[0].pubkey;

    let tx = Transaction::new_signed_with_payer(
        vec_of_instructions.as_slice(), 
        Some(payer), 
        &[signer], 
        rpc_client.get_latest_blockhash().await?
    );

     

    // let tx = Transaction::new(from_keypairs, message, recent_blockhash)

    // let message = Message::new_with_compiled_instructions(num_required_signatures, num_readonly_signed_accounts, num_readonly_unsigned_accounts, account_keys, recent_blockhash, instructions)

    // Create proof transaction, calling the right function in the contract

    // Send transaction to contract on chain
    let settle_tx_hash = rpc_client
        .send_and_confirm_transaction(&tx)
        .await?;

    log::info!("Settled state: {}", settle_tx_hash.to_string());
    Ok(settle_tx_hash.to_string())
}

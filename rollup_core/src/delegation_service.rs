use {
    crate::delegation::{create_delegation_instruction, create_withdrawal_instruction, find_delegation_pda, DelegatedAccount, create_topup_instruction}, anyhow::{anyhow, Result}, borsh::BorshDeserialize, log, solana_client::rpc_client::RpcClient, solana_sdk::{
        account::{AccountSharedData, ReadableAccount}, message::Message, pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction
    }, std::collections::HashMap
};

pub struct DelegationService {
    rpc_client: RpcClient,
    pda_cache: HashMap<Pubkey, AccountSharedData>,
    signer: Keypair,
}

impl DelegationService {
    pub fn new(rpc_url: &str, signer: Keypair) -> Self {
        Self {
            rpc_client: RpcClient::new(rpc_url.to_string()),
            pda_cache: HashMap::new(),
            signer,
        }
    }

    pub fn get_or_fetch_pda(&mut self, user: &Pubkey) -> Result<Option<(Pubkey, DelegatedAccount)>> {
        let (pda, _) = find_delegation_pda(user);
        
        // Always try fetching from chain first to be sure
        match self.rpc_client.get_account(&pda) {
            Ok(account) => {
                log::info!(
                    "Found account for PDA: {}, data length: {}, owner: {}", 
                    pda,
                    account.data().len(),
                    account.owner()
                );
                // Skip the 8-byte discriminator when deserializing
                if account.data().len() > 8 {
                    if let Ok(delegation) = DelegatedAccount::try_from_slice(&account.data()[8..]) {
                        self.pda_cache.insert(pda, account.into());
                        Ok(Some((pda, delegation)))
                    } else {
                        log::warn!(
                            "Account exists but couldn't deserialize data for PDA: {}. Data: {:?}", 
                            pda,
                            account.data()
                        );
                        Ok(None)
                    }
                } else {
                    log::warn!("Account data too short for PDA: {}", pda);
                    Ok(None)
                }
            }
            Err(e) => {
                log::info!("No account found for PDA: {} (Error: {})", pda, e);
                self.pda_cache.remove(&pda);
                Ok(None)
            }
        }
    }

    pub fn verify_delegation_for_transaction(
        &mut self,
        user: &Pubkey,
        required_amount: u64,
    ) -> Result<Option<Pubkey>> {
        if let Some((pda, delegation)) = self.get_or_fetch_pda(user)? {
            log::info!(
                "Verifying delegation for {}: current={}, required={}", 
                user, 
                delegation.delegated_amount, 
                required_amount
            );
            if delegation.delegated_amount >= required_amount {
                Ok(Some(pda))
            } else {
                Ok(None)
            }
        } else {
            log::info!("No delegation found for {}", user);
            Ok(None)
        }
    }

    pub fn create_delegation_transaction(
        &mut self,
        user: &Pubkey,
        amount: u64,
    ) -> Result<Transaction> {
        let instruction = if let Some((_, delegation)) = self.get_or_fetch_pda(user)? {
            log::info!(
                "Found existing delegation for {} with amount {}. Creating top-up instruction.", 
                user,
                delegation.delegated_amount
            );
            create_topup_instruction(user, amount)
        } else {
            create_delegation_instruction(user, amount)
        };

        let recent_blockhash = self.rpc_client.get_latest_blockhash()?;
        
        let message = Message::new_with_blockhash(
            &[instruction],
            Some(&self.signer.pubkey()),
            &recent_blockhash
        );
        
        let mut tx = Transaction::new_unsigned(message);
        tx.try_sign(&[&self.signer], recent_blockhash)?;
        
        Ok(tx)
    }

    pub fn update_pda_state(&mut self, pda: Pubkey, account: AccountSharedData) {
        self.pda_cache.insert(pda, account);
    }


    pub fn create_withdrawal_transaction(&mut self, pda: &Pubkey, owner: &Pubkey, amount: u64) -> Result<Transaction> {
        let instruction = create_withdrawal_instruction(pda, owner, amount);
        
        let recent_blockhash = self.rpc_client.get_latest_blockhash()?;
        let message = Message::new(&[instruction], Some(&self.signer.pubkey()));
        
        let mut tx = Transaction::new_unsigned(message);
        tx.sign(&[&self.signer], recent_blockhash);
        
        Ok(tx)
    }
} 

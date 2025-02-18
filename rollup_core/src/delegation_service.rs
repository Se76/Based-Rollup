use {
    crate::delegation::{find_delegation_pda, DelegatedAccount, create_delegation_instruction},
    solana_sdk::{
        account::{AccountSharedData, ReadableAccount},
        pubkey::Pubkey,
        transaction::Transaction,
        message::Message,
    },
    solana_client::rpc_client::RpcClient,
    anyhow::{Result, anyhow},
    std::collections::HashMap,
    borsh::BorshDeserialize,
};

pub struct DelegationService {
    rpc_client: RpcClient,
    pda_cache: HashMap<Pubkey, AccountSharedData>,
}

impl DelegationService {
    pub fn new(rpc_url: &str) -> Self {
        Self {
            rpc_client: RpcClient::new(rpc_url.to_string()),
            pda_cache: HashMap::new(),
        }
    }

    pub fn get_or_fetch_pda(&mut self, user: &Pubkey) -> Result<Option<(Pubkey, DelegatedAccount)>> {
        let (pda, _) = find_delegation_pda(user);
        
        // Try cache first
        if let Some(account) = self.pda_cache.get(&pda) {
            if let Ok(delegation) = DelegatedAccount::try_from_slice(&account.data()) {
                return Ok(Some((pda, delegation)));
            }
        }

        // If not in cache, try fetching from chain
        match self.rpc_client.get_account(&pda) {
            Ok(account) => {
                if let Ok(delegation) = DelegatedAccount::try_from_slice(&account.data()) {
                    self.pda_cache.insert(pda, account.into());
                    Ok(Some((pda, delegation)))
                } else {
                    Ok(None)
                }
            }
            Err(_) => Ok(None)
        }
    }

    pub fn verify_delegation_for_transaction(
        &mut self,
        user: &Pubkey,
        required_amount: u64,
    ) -> Result<Option<Pubkey>> {  // Returns PDA if delegation exists and is sufficient
        if let Some((pda, delegation)) = self.get_or_fetch_pda(user)? {
            if delegation.delegated_amount >= required_amount {
                Ok(Some(pda))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub fn create_delegation_transaction(
        &self,
        user: &Pubkey,
        amount: u64,
    ) -> Result<Transaction> {
        let instruction = create_delegation_instruction(user, amount);
        let recent_blockhash = self.rpc_client.get_latest_blockhash()?;
        
        Ok(Transaction::new_with_payer(
            &[instruction],
            Some(user),
        ))
    }

    pub fn update_pda_state(&mut self, pda: Pubkey, account: AccountSharedData) {
        self.pda_cache.insert(pda, account);
    }
} 

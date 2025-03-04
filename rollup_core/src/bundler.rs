use std::collections::HashMap;
use solana_sdk::{instruction::{CompiledInstruction, Instruction}, program_pack::Pack, pubkey::Pubkey, system_instruction::{self, SystemInstruction}, system_program, transaction::Transaction};
use bincode::deserialize;
use solana_client::rpc_client::RpcClient;
use spl_token::state::Account as TokenAccount;

pub fn get_transaction_instructions(tx: &Transaction) -> Vec<CompiledInstruction>{
    tx.message.instructions.clone()
}

pub fn is_transfer_ix(cix: &CompiledInstruction, account_keys: &[Pubkey]) -> bool {
    if cix.program_id_index as usize >= account_keys.len(){
        return false;
    }
    let program_id = account_keys[cix.program_id_index as usize];
    if program_id != system_program::ID{
        return false
    }
    
    matches!(
        deserialize::<SystemInstruction>(&cix.data),
        Ok(SystemInstruction::Transfer { .. })
    )
}

pub fn is_token_transfer_ix(cix: &CompiledInstruction, account_keys: &[Pubkey]) -> bool {
    if cix.program_id_index as usize >= account_keys.len(){
        return false;
    }
    let program_id = account_keys[cix.program_id_index as usize];
    if program_id != spl_token::ID{
        return false;
    }
    !cix.data.is_empty() && (cix.data[0] == 3 || cix.data[0] == 12)
}

fn fetch_mint_from_rpc(token_account: &Pubkey, rpc_client: &RpcClient) -> Option<Pubkey> {
    let acc_info = rpc_client.get_account_data(token_account).ok()?;
    let token_account = TokenAccount::unpack(&acc_info).ok()?;
    Some(token_account.mint)
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct TBundlerKey {
    keys: [Pubkey; 2],
    mint: Pubkey
}

pub struct TransferBundler {
    transfers: HashMap<TBundlerKey, i128>,
    authorities: HashMap<Pubkey, Pubkey>,
    rpc_client: RpcClient
}

impl TransferBundler {
    pub fn new(rpc_client: RpcClient) -> Self {
        Self {
            transfers: HashMap::new(),
            authorities: HashMap::new(),
            rpc_client
        }
    }

    
    //Parses transactions and add transfer ixs to TransferBundler
    pub fn bundle(&mut self, transaction: Transaction){
        let ixs = get_transaction_instructions(&transaction);
        let account_keys: &[Pubkey] = &transaction.message.account_keys;
        for ix in ixs {
            if let Some((from, to, mint, amount)) = Self::parse_token_transfer(&ix, account_keys, &self.rpc_client){
                let mut keys = [from, to];
                keys.sort();
                *self.transfers.entry(TBundlerKey {keys, mint}).or_default() += if from == keys[0] {amount} else {-amount};
                self.authorities
                    .entry(from.clone())
                    .or_insert(if ix.data[0] == 12 { account_keys[ix.accounts[3] as usize] } else { account_keys[ix.accounts[2] as usize] });
            }
        }
    }

    pub fn parse_token_transfer(ix: &CompiledInstruction, account_keys: &[Pubkey], rpc_client: &RpcClient) -> Option<(Pubkey, Pubkey, Pubkey, i128)> {
        if !is_token_transfer_ix(ix, account_keys) || ix.accounts.len() < 2{
            return None;
        }

        log::info!("PTT accounts: {:#?}", ix.accounts);
        let from = account_keys[ix.accounts[0] as usize];
        let mut to = Pubkey::default();
        let mut mint = Pubkey::default();

        if ix.data[0] == 12 {
            to = account_keys[ix.accounts[2] as usize];
            mint = account_keys[ix.accounts[1] as usize];
        } else {
            to = account_keys[ix.accounts[1] as usize];
            mint = fetch_mint_from_rpc(&from, rpc_client)?;
        }
        
        
        let amount = u64::from_le_bytes(ix.data[1..9].try_into().ok()?);
        Some((from, to, mint, amount as i128))
    }

    pub fn generate_final(self) -> Vec<Instruction> {
        // SOL IMPLEMENTATION
        // self.transfers.into_iter().filter_map(|(map_key, val)| {
        //     if val < 0 {
        //         Some(system_instruction::transfer(&map_key.keys[1], &map_key.keys[0], val.unsigned_abs() as u64))
        //     } else if val > 0 {
        //         Some(system_instruction::transfer(&map_key.keys[0], &map_key.keys[1], val as u64))
        //     } else {
        //         None
        //     }
        // }).collect()

        // SPL IMPLEMENTATION:
        let txs: Vec<Instruction> = self.transfers.into_iter().filter_map(|(map_key, val)| {
            if val == 0{
                return None;
            } 

            let from = map_key.keys[if val < 0 {1} else {0}];
            let to = map_key.keys[if val < 0 {0} else {1}];
            let amount = val.unsigned_abs() as u64;

            Some(spl_token::instruction::transfer(
                &spl_token::ID, 
                &from, 
                &to, 
                self.authorities.get(&from).unwrap(), 
                &[], 
                amount
            ).ok()?)
        }).collect();

        for tx in &txs{
            log::info!("{:#?}", tx);
        }

        txs
    }
}


pub fn parse_compiled_instruction(ix: &CompiledInstruction, account_keys: &[Pubkey]) -> Option<(Pubkey, Pubkey, i128)>{
    //Ensure the instruction is from System Program (where transfer is from)
    if ix.program_id_index as usize >= account_keys.len()  || account_keys[ix.program_id_index as usize] != system_program::ID{
        return None;
    }
    //Ensure we have at least 2 accounts for transfer and enough data for SOL amount
    if ix.accounts.len() < 2 || ix.data.len() < 8 {
        return None;
    }

    //Get accounts involved in transfer and amount to be transferred
    let from = account_keys[ix.accounts[0] as usize];
    let to = account_keys[ix.accounts[1] as usize];

    log::info!("FROM: {:?}", from.to_string());
    log::info!("TO: {:?}", to.to_string());
    log::info!("IX DATA: {:?}", ix.data);

    let amount = u64::from_le_bytes(ix.data[4..12].try_into().ok()?);
    Some((from, to, amount as i128))
}

pub fn parse_instruction(ix: &Instruction) -> Option<(Pubkey, Pubkey, i128)>{
    //Enusre ix is owned by system program
    if ix.program_id != system_program::ID{
        return None;
    }

    //Ensure we have enough accounts
    if ix.accounts.len() < 2{
        return None;
    }
    let from = ix.accounts[0].pubkey;
    let to = ix.accounts[1].pubkey;
    let amount = u64::from_le_bytes(ix.data[4..].try_into().ok()?);
    
    log::info!("FROM: {:?}", from.to_string());
    log::info!("TO: {:?}", to.to_string());
    log::info!("AMOUNT: {amount}");
    log::info!("IX DATA: {:?}", ix.data);

    
    Some((from, to, amount as i128))
}

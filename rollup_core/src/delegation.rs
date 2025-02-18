use solana_sdk::{
    pubkey::Pubkey,
    instruction::{AccountMeta, Instruction},
    system_program,
};
use borsh::{BorshSerialize, BorshDeserialize};

#[derive(BorshSerialize, BorshDeserialize)]
pub enum DelegationInstruction {
    InitializeDelegate { amount: u64 },
    Withdraw { amount: u64 },
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct DelegatedAccount {
    pub owner: Pubkey,
    pub delegated_amount: u64,
    pub last_deposit_time: i64,
    pub bump: u8,
}

pub fn get_delegation_program_id() -> Pubkey {
    "E1bxy4HwKFjPARhVe7NjvoFtynN69C4xNA53uSwruHrP"
        .parse()
        .unwrap()
}

pub fn find_delegation_pda(owner: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"delegate", owner.as_ref()],
        &get_delegation_program_id()
    )
}

pub fn create_delegation_instruction(
    owner: &Pubkey,
    amount: u64,
) -> Instruction {
    let (pda, _) = find_delegation_pda(owner);
    
    Instruction {
        program_id: get_delegation_program_id(),
        accounts: vec![
            AccountMeta::new(*owner, true),
            AccountMeta::new(pda, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: borsh::to_vec(&DelegationInstruction::InitializeDelegate { amount }).unwrap(),
    }
} 
use solana_sdk::{
    pubkey::Pubkey,
    instruction::{AccountMeta, Instruction},
    system_program,
};
use borsh::{BorshSerialize, BorshDeserialize};

#[derive(BorshSerialize, BorshDeserialize)]
pub struct DelegatedAccount {
    pub owner: Pubkey,
    pub delegated_amount: u64,
    pub last_deposit_time: i64,
    pub bump: u8,
}

#[derive(BorshSerialize)]
struct AnchorInstruction {
    discriminator: [u8; 8],
    data: InitializeDelegateArgs,
}

#[derive(BorshSerialize)]
pub struct InitializeDelegateArgs {
    pub amount: u64,
}

pub fn get_delegation_program_id() -> Pubkey {
    "5MSF4TiUfD7dVm7P1ahPYJfEBLCUQn7hEPYXYHocVwzh"
        .parse()
        .unwrap()
}

pub fn find_delegation_pda(owner: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"delegate", owner.as_ref()],
        &get_delegation_program_id()
    )
}

pub fn create_delegation_instruction(owner: &Pubkey, amount: u64) -> Instruction {
    let (pda, _) = find_delegation_pda(owner);
    
    // Anchor discriminator for "initialize_delegate"
    let discriminator = [103, 117, 89, 87, 161, 37, 220, 226];
    
    let ix_data = AnchorInstruction {
        discriminator,
        data: InitializeDelegateArgs { amount },
    }.try_to_vec().unwrap();

    Instruction {
        program_id: get_delegation_program_id(),
        accounts: vec![
            AccountMeta::new(*owner, true),
            AccountMeta::new(pda, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: ix_data,
    }
} 
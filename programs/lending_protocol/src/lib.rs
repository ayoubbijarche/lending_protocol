use anchor_lang::prelude::*;
use anchor_spl::token::Token;

declare_id!("79AoAxGPczqpkzg5ygo7rthSTh4DA7UZgFjyHocSPT2f");

#[program]
pub mod lending_protocol {
    use super::*;
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {

}

#[derive(Accounts)]
pub struct Lend<'info>{
    #[account(
        init,
        payer = signer,
        space = 8 + LendingData::InitSpace
    )]
    pub lending_acc : Account<'info , LendingData>,
    #[account(mut)]
    pub token_account : AccountInfo<'info>,
    pub signer : Signer<'info>,
    pub token_program : Program<'info , Token>,
    pub system_program : Program<'info , System>
}


#[account]
#[derive(InitSpace)]
pub struct LendingData{
    pub client : Pubkey,
    pub token_account : Pubkey,
    pub amount : u64,
    pub interest_rate : u32,
}


pub enum LendingErr{
    NoAssetsErr,
    NoRequirmentMet
}





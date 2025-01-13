use anchor_lang::prelude::*;
use anchor_spl::token::Token;

declare_id!("79AoAxGPczqpkzg5ygo7rthSTh4DA7UZgFjyHocSPT2f");

#[program]
pub mod lending_protocol {


    use anchor_lang::system_program;
    use anchor_spl::token;

    use super::*;
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }

    pub fn lend_money(ctx : Context<Lend> , amount : u64) -> Result<()> {
        let lending_acc = &mut ctx.accounts.lending_acc;
        lending_acc.amount = amount;
        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer{
                    from : ctx.accounts.lender.to_account_info(),
                    to : ctx.accounts.client.to_account_info(),
                }
            ),
            lending_acc.amount
        )?;
        

        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {                    
                    from: ctx.accounts.client_token_account.to_account_info(),
                    to: ctx.accounts.lender_token_account.to_account_info(),
                    authority: ctx.accounts.client.to_account_info(),
                },
                &[]
                ),
            1
        )?;
        Ok(())
    }
    
    pub fn payment(ctx : Context<Lend> , amount : u64 , interest : u64) -> Result<()>{
        
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
        payer = lender,
        space = 8 + LendingData::INIT_SPACE
    )]
    pub lending_acc : Account<'info , LendingData>,
    #[account(mut)]
    pub client : Signer<'info>,
    #[account(mut)]
    pub lender : SystemAccount<'info>,
    pub lender_token_account : AccountInfo<'info>,
    pub client_token_account : AccountInfo<'info>,
    pub token_program : Program<'info , Token>,
    pub system_program : Program<'info , System>
}


#[account]
#[derive(InitSpace)]
pub struct LendingData{
    pub client : Pubkey,
    pub token_account : Pubkey,
    pub amount : u64,
    pub interest_rate : u64,
}


pub enum LendingErr{
    NoAssetsErr,
    NoRequirmentMet
}





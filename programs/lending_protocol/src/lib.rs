use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, Mint, TokenAccount};
use anchor_lang::system_program;

declare_id!("79AoAxGPczqpkzg5ygo7rthSTh4DA7UZgFjyHocSPT2f");

#[program]
pub mod lending_protocol {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Lending protocol initialized: {:?}", ctx.program_id);
        Ok(())
    }

    pub fn lend_money(ctx: Context<Lend>, amount: u64, duration_months: u8) -> Result<()> {
        require!(amount > 0, LendingError::InvalidAmount);
        require!(duration_months > 0 && duration_months <= 60, LendingError::InvalidDuration);
        require_gt!(ctx.accounts.lender.lamports(), amount, LendingError::InsufficientLenderFunds);

        let lending_acc = &mut ctx.accounts.lending_acc;
        
        lending_acc.interest_rate = if amount <= 5 * LAMPORTS_PER_SOL {
            2
        } else if amount <= 20 * LAMPORTS_PER_SOL {
            10
        } else {
            15
        };

        lending_acc.amount = amount;
        lending_acc.remaining_amount = amount;
        lending_acc.duration_months = duration_months;
        lending_acc.monthly_payment = calculate_monthly_payment(amount, duration_months, lending_acc.interest_rate);
        lending_acc.last_payment_timestamp = Clock::get()?.unix_timestamp;
        lending_acc.start_time = Clock::get()?.unix_timestamp;
        lending_acc.client = ctx.accounts.client.key();
        lending_acc.lender = ctx.accounts.lender.key();
        lending_acc.collateral_token_mint = ctx.accounts.mint.key();
        lending_acc.is_active = true;
        lending_acc.bump = ctx.bumps.lending_acc;

        // Transfer loan amount
        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.lender.to_account_info(),
                    to: ctx.accounts.client.to_account_info(),
                }
            ),
            amount
        )?;

        // Transfer collateral
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.client_token_account.to_account_info(),
                    to: ctx.accounts.lender_token_account.to_account_info(),
                    authority: ctx.accounts.client.to_account_info(),
                }
            ),
            1
        )?;

        emit!(LoanCreated {
            client: ctx.accounts.client.key(),
            lender: ctx.accounts.lender.key(),
            amount,
            duration_months,
            monthly_payment: lending_acc.monthly_payment,
            interest_rate: lending_acc.interest_rate,
        });

        Ok(())
    }

    pub fn make_payment(ctx: Context<MakePayment>) -> Result<()> {
        let lending_acc = &mut ctx.accounts.lending_acc;
        require!(lending_acc.is_active, LendingError::LoanInactive);
        
        let current_time = Clock::get()?.unix_timestamp;
        let time_since_last_payment = current_time - lending_acc.last_payment_timestamp;

        require!(
            time_since_last_payment >= 28 * 24 * 60 * 60 && 
            time_since_last_payment <= 31 * 24 * 60 * 60,
            LendingError::InvalidPaymentTiming
        );

        require_gt!(
            ctx.accounts.client.to_account_info().lamports(),
            lending_acc.monthly_payment,
            LendingError::InsufficientFunds
        );

        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.client.to_account_info(),
                    to: ctx.accounts.lender.to_account_info(),
                }
            ),
            lending_acc.monthly_payment
        )?;

        lending_acc.remaining_amount = lending_acc.remaining_amount.saturating_sub(
            lending_acc.monthly_payment.saturating_sub(
                lending_acc.monthly_payment * lending_acc.interest_rate / 100
            )
        );
        lending_acc.last_payment_timestamp = current_time;

        if lending_acc.remaining_amount == 0 {
            lending_acc.is_active = false;

            token::transfer(
                CpiContext::new(
                    ctx.accounts.token_program.to_account_info(),
                    token::Transfer {
                        from: ctx.accounts.lender_token_account.to_account_info(),
                        to: ctx.accounts.client_token_account.to_account_info(),
                        authority: ctx.accounts.lender.to_account_info(),
                    }
                ),
                1
            )?;

            emit!(LoanCompleted {
                client: ctx.accounts.client.key(),
                lender: ctx.accounts.lender.key(),
                total_paid: lending_acc.amount + 
                    (lending_acc.amount * lending_acc.interest_rate / 100),
            });
        }

        emit!(PaymentMade {
            client: ctx.accounts.client.key(),
            amount: lending_acc.monthly_payment,
            remaining: lending_acc.remaining_amount,
            payment_number: (current_time - lending_acc.start_time) / (30 * 24 * 60 * 60),
        });

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}


#[derive(Accounts)]
#[instruction(amount: u64, duration_months: u8)]
pub struct Lend<'info> {
    #[account(
        init,
        payer = lender,
        space = 8 + LendingData::INIT_SPACE,
        seeds = [b"lending", lender.key().as_ref(), client.key().as_ref()],
        bump
    )]
    pub lending_acc: Account<'info, LendingData>,
    
    #[account(mut)]
    pub client: Signer<'info>,
    
    #[account(mut)]
    pub lender: Signer<'info>,
    
    /// CHECK: This is the mint of the token being used as collateral
    pub mint: AccountInfo<'info>,

    #[account(
        mut,
        constraint = lender_token_account.owner == lender.key(),
        constraint = lender_token_account.mint == mint.key()
    )]
    pub lender_token_account: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        constraint = client_token_account.owner == client.key(),
        constraint = client_token_account.mint == mint.key()
    )]
    pub client_token_account: Account<'info, TokenAccount>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct MakePayment<'info> {
    #[account(
        mut,
        seeds = [b"lending", lender.key().as_ref(), client.key().as_ref()],
        bump = lending_acc.bump,
        has_one = client,
        has_one = lender,
        constraint = lending_acc.is_active @ LendingError::LoanInactive
    )]
    pub lending_acc: Account<'info, LendingData>,
    
    #[account(mut)]
    pub client: Signer<'info>,
    
    /// CHECK: This is the lender's account that will receive the payment
    #[account(mut)]
    pub lender: AccountInfo<'info>,
    
    #[account(
        mut,
        constraint = lender_token_account.owner == lender.key(),
        constraint = lender_token_account.mint == mint.key()
    )]
    pub lender_token_account: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        constraint = client_token_account.owner == client.key(),
        constraint = client_token_account.mint == mint.key()
    )]
    pub client_token_account: Account<'info, TokenAccount>,
    
    /// CHECK: This is the mint of the token being used as collateral
    pub mint: AccountInfo<'info>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}


#[account]
#[derive(InitSpace)]
pub struct LendingData {
    pub client: Pubkey,
    pub lender: Pubkey,
    pub collateral_token_mint: Pubkey,
    pub amount: u64,
    pub remaining_amount: u64,
    pub interest_rate: u64,
    pub duration_months: u8,
    pub monthly_payment: u64,
    pub last_payment_timestamp: i64,
    pub start_time: i64,
    pub is_active: bool,
    pub bump: u8,
}

#[error_code]
pub enum LendingError {
    #[msg("Invalid loan amount")]
    InvalidAmount,
    #[msg("Invalid loan duration (must be between 1-60 months)")]
    InvalidDuration,
    #[msg("Insufficient collateral")]
    InsufficientCollateral,
    #[msg("Insufficient funds for payment")]
    InsufficientFunds,
    #[msg("Invalid payment timing")]
    InvalidPaymentTiming,
    #[msg("Loan is no longer active")]
    LoanInactive,
    #[msg("Lender has insufficient funds")]
    InsufficientLenderFunds,
}

#[event]
pub struct LoanCreated {
    pub client: Pubkey,
    pub lender: Pubkey,
    pub amount: u64,
    pub duration_months: u8,
    pub monthly_payment: u64,
    pub interest_rate: u64,
}

#[event]
pub struct PaymentMade {
    pub client: Pubkey,
    pub amount: u64,
    pub remaining: u64,
    pub payment_number: i64,
}

#[event]
pub struct LoanCompleted {
    pub client: Pubkey,
    pub lender: Pubkey,
    pub total_paid: u64,
}

const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

fn calculate_monthly_payment(amount: u64, duration_months: u8, interest_rate: u64) -> u64 {
    let total_interest = amount * interest_rate / 100;
    let total_amount = amount + total_interest;
    total_amount / duration_months as u64
}



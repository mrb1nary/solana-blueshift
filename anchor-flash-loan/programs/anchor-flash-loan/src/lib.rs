use anchor_lang::prelude::*;
use anchor_lang::{
    solana_program::sysvar::instructions::{
        load_instruction_at_checked, ID as INSTRUCTIONS_SYSVAR_ID,
    },
    Discriminator,
};
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{transfer, Mint, Token, TokenAccount, Transfer},
};

declare_id!("22222222222222222222222222222222222222222222");

#[program]
pub mod blueshift_anchor_flash_loan {
    use super::*;

    pub fn borrow(ctx: Context<Loan>, borrow_amount: u64) -> Result<()> {
        // Make sure we're not sending in an invalid amount that can crash our Protocol
        require!(borrow_amount > 0, ProtocolError::InvalidAmount);

        // Derive the signer seeds for the Protocol Account
        let seeds = &[b"protocol".as_ref(), &[ctx.bumps.protocol]];
        let signer_seeds = &[&seeds[..]];

        // Transfer the funds from the protocol to the borrower
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.protocol_ata.to_account_info(),
                    to: ctx.accounts.borrower_ata.to_account_info(),
                    authority: ctx.accounts.protocol.to_account_info(),
                },
                signer_seeds,
            ),
            borrow_amount,
        )?;

        // Let's introspect the txn

        let ixs = ctx.accounts.instructions.to_account_info();

        

        // Check how many ix we have in this tx
        let instruction_sysvar = ixs.try_borrow_data()?;
        let len = u16::from_le_bytes(instruction_sysvar[0..2].try_into().unwrap());

        // Ensure we have a repay ix
        if let Ok(repay_ix) = load_instruction_at_checked(len as usize - 1, &ixs) {
            // Instruction checks
            require_keys_eq!(repay_ix.program_id, ID, ProtocolError::InvalidProgram);
            require!(
                repay_ix.data[0..8].eq(instruction::Repay::DISCRIMINATOR),
                ProtocolError::InvalidIx
            );

            // We could check the Wallet and Mint separately but by checking the ATA we do this automatically
            require_keys_eq!(
                repay_ix
                    .accounts
                    .get(3)
                    .ok_or(ProtocolError::InvalidBorrowerAta)?
                    .pubkey,
                ctx.accounts.borrower_ata.key(),
                ProtocolError::InvalidBorrowerAta
            );
            require_keys_eq!(
                repay_ix
                    .accounts
                    .get(4)
                    .ok_or(ProtocolError::InvalidProtocolAta)?
                    .pubkey,
                ctx.accounts.protocol_ata.key(),
                ProtocolError::InvalidProtocolAta
            );
        } else {
            return Err(ProtocolError::MissingRepayIx.into());
        }

        Ok(())
    }

    pub fn repay(ctx: Context<Loan>) -> Result<()> {
        // Repayment logicccccccc

        let ixs = ctx.accounts.instructions.to_account_info();

        let mut amount_borrowed: u64;

        if let Ok(borrow_ix) = load_instruction_at_checked(0, &ixs) {
            // Check the amount borrowed:
            let mut borrowed_data: [u8; 8] = [0u8; 8];
            borrowed_data.copy_from_slice(&borrow_ix.data[8..16]);
            amount_borrowed = u64::from_le_bytes(borrowed_data)
        } else {
            return Err(ProtocolError::MissingBorrowIx.into());
        }

        // Add the fee to the amount borrowed (In our case we hardcoded it to 500 basis point)
        let fee = (amount_borrowed as u128)
            .checked_mul(500)
            .unwrap()
            .checked_div(10_000)
            .ok_or(ProtocolError::Overflow)? as u64;
        amount_borrowed = amount_borrowed
            .checked_add(fee)
            .ok_or(ProtocolError::Overflow)?;

        // Transfer the funds from the protocol to the borrower
        transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.borrower_ata.to_account_info(),
                    to: ctx.accounts.protocol_ata.to_account_info(),
                    authority: ctx.accounts.borrower.to_account_info(),
                },
            ),
            amount_borrowed,
        )?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Loan<'info> {
    #[account(mut)]
    pub borrower: Signer<'info>,        
    #[account(
    seeds = [b"protocol".as_ref()],
    bump,
  )]
    pub protocol: SystemAccount<'info>,

    pub mint: Account<'info, Mint>,
    #[account(
    init_if_needed,
    payer = borrower,
    associated_token::mint = mint,
    associated_token::authority = borrower,
  )]
    pub borrower_ata: Account<'info, TokenAccount>,
    #[account(
    mut,
    associated_token::mint = mint,
    associated_token::authority = protocol,
  )]
    pub protocol_ata: Account<'info, TokenAccount>,

    #[account(address = INSTRUCTIONS_SYSVAR_ID)]
    /// CHECK: InstructionsSysvar account
    instructions: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[error_code]
pub enum ProtocolError {
    #[msg("Invalid instruction")]
    InvalidIx,
    #[msg("Invalid instruction index")]
    InvalidInstructionIndex,
    #[msg("Invalid amount")]
    InvalidAmount,
    #[msg("Not enough funds")]
    NotEnoughFunds,
    #[msg("Program Mismatch")]
    ProgramMismatch,
    #[msg("Invalid program")]
    InvalidProgram,
    #[msg("Invalid borrower ATA")]
    InvalidBorrowerAta,
    #[msg("Invalid protocol ATA")]
    InvalidProtocolAta,
    #[msg("Missing repay instruction")]
    MissingRepayIx,
    #[msg("Missing borrow instruction")]
    MissingBorrowIx,
    #[msg("Overflow")]
    Overflow,
}
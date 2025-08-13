use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    msg,
    program_error::ProgramError,
    pubkey::create_program_address,
    ProgramResult,
};
use pinocchio_token::instructions::{CloseAccount, Transfer};

use crate::{
    AccountCheck, AccountClose, AssociatedTokenAccount, AssociatedTokenAccountInit, Escrow,
    MintInterface, ProgramAccount, SignerAccount, TokenAccount,
};

pub struct RefundAccounts<'a> {
    pub maker: &'a AccountInfo,
    pub escrow: &'a AccountInfo,
    pub mint_a: &'a AccountInfo,
    pub vault: &'a AccountInfo,
    pub maker_ata_a: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
    pub token_program: &'a AccountInfo,
    pub associated_token_account_program: &'a AccountInfo
}

impl<'a> TryFrom<&'a [AccountInfo]> for RefundAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        // Check if all accounts have been inputed
        msg!(&format!("Accounts length: {}", accounts.len()));

        let [maker, escrow, mint_a, vault, maker_ata_a, system_program, token_program, associated_token_account_program] =
            accounts
        else {
            msg!("TEST123");
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // Basic accounts checks
        msg!("SignerAccount");
        SignerAccount::check(maker)?;
        msg!("MintInterface A");
        MintInterface::check(mint_a)?;

        Ok(Self {
            maker,
            escrow,
            mint_a,
            vault,
            maker_ata_a,
            system_program,
            token_program,
            associated_token_account_program
        })
    }
}

pub struct Refund<'a> {
    pub accounts: RefundAccounts<'a>,
}

impl<'a> TryFrom<&'a [AccountInfo]> for Refund<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let accounts = RefundAccounts::try_from(accounts)?;

        msg!("INIT_IF_NEEDED");
        // init_if_needed maker ata a
        AssociatedTokenAccount::init_if_needed(
            accounts.maker_ata_a,
            accounts.mint_a,
            accounts.maker,
            accounts.maker,
            &accounts.system_program,
            &accounts.token_program,
        )?;

        Ok(Self { accounts })
    }
}

impl<'a> Refund<'a> {
    pub const DISCRIMINATOR: &'a u8 = &2;

    pub fn process(&mut self) -> ProgramResult {
        // Populate escrow account
        let data = self.accounts.escrow.try_borrow_data()?;
        let escrow = Escrow::load(&data)?;

        msg!("Program create");
        let escrow_key = create_program_address(
            &[
                b"escrow",
                self.accounts.maker.key(),
                &escrow.seed.to_le_bytes(),
                &escrow.bump,
            ],
            &crate::ID,
        )?;

        if &escrow_key != self.accounts.escrow.key() {
            msg!("Check escrow key");
            return Err(ProgramError::InvalidAccountOwner);
        }

        let seed_binding = escrow.seed.to_le_bytes();
        let bump_binding = escrow.bump;
        let escrow_seeds = [
            Seed::from(b"escrow"),
            Seed::from(self.accounts.maker.key().as_ref()),
            Seed::from(&seed_binding),
            Seed::from(&bump_binding),
        ];
        let signer = Signer::from(&escrow_seeds);

        msg!(&format!("vault.owner: {:?}", unsafe { self.accounts.vault.owner() }));
        msg!(&format!("vault.key: {:?}", self.accounts.vault.key()));
        msg!(&format!("vault.data_len: {}", self.accounts.vault.data_len()));

        msg!("Get amount");
        let amount = TokenAccount::get_amount(self.accounts.vault)?;

        msg!("Transfer");
        // Transfer from Vault to maker
        Transfer {
            from: self.accounts.vault,
            to: self.accounts.maker_ata_a,
            authority: self.accounts.escrow,
            amount,
        }
        .invoke_signed(&[signer.clone()])?;

        msg!("Close vault");
        // Close vault
        CloseAccount {
            account: self.accounts.vault,
            destination: self.accounts.maker,
            authority: self.accounts.escrow,
        }
        .invoke_signed(&[signer.clone()])?;

        msg!("Close vault");
        // Close escrow
        drop(data);
        ProgramAccount::close(self.accounts.escrow, self.accounts.maker)?;

        Ok(())
    }
}
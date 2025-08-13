use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    msg,
    program_error::ProgramError,
    pubkey::create_program_address,
    ProgramResult,
};
use pinocchio_token::instructions::{CloseAccount, Transfer};

use crate::helpers::MintInterface;
use crate::helpers::ProgramAccount;
use crate::helpers::SignerAccount;
use crate::helpers::TokenAccount;
use crate::helpers::{
    AccountCheck, AccountClose, AssociatedTokenAccount, AssociatedTokenAccountCheck,
    AssociatedTokenAccountInit,
};
use crate::Escrow;

pub struct TakeAccounts<'a> {
    pub taker: &'a AccountInfo,
    pub maker: &'a AccountInfo,
    pub escrow: &'a AccountInfo,
    pub mint_a: &'a AccountInfo,
    pub mint_b: &'a AccountInfo,
    pub vault: &'a AccountInfo,
    pub taker_ata_a: &'a AccountInfo,
    pub taker_ata_b: &'a AccountInfo,
    pub maker_ata_b: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
    pub token_program: &'a AccountInfo,
}

// Checks for Take instructions accounts
impl<'a> TryFrom<&'a [AccountInfo]> for TakeAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        // Deserializing the accounts
        let [taker, maker, escrow, mint_a, mint_b, vault, taker_ata_a, taker_ata_b, maker_ata_b, system_program, token_program, _] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // Basic checks
        msg!("CHECK: SignerAccount taker");
        SignerAccount::check(taker)?;
        msg!("CHECK: ProgramAccount escrow");
        ProgramAccount::check(escrow)?;
        msg!("CHECK: MintInterface mint_a");
        MintInterface::check(mint_a)?;
        msg!("CHECK: MintInterface mint_b");
        MintInterface::check(mint_b)?;
        msg!("CHECK: ATA taker B");
        AssociatedTokenAccount::check(taker_ata_b, taker, mint_b)?;
        msg!("CHECK: PDA vault");
        AssociatedTokenAccount::check(vault, escrow, mint_a)?;

        // Return the accounts after checks
        Ok(Self {
            taker,
            maker,
            escrow,
            mint_a,
            mint_b,
            vault,
            taker_ata_a,
            taker_ata_b,
            maker_ata_b,
            system_program,
            token_program,
        })
    }
}

pub struct Take<'a> {
    pub accounts: TakeAccounts<'a>,
}

// Implement checks for the accounts and init_if_needed for neccesary ATA's
impl<'a> TryFrom<&'a [AccountInfo]> for Take<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        // Pass the accounts through the check
        let accounts = TakeAccounts::try_from(accounts)?;

        // initialize neccessary mint_a ATA accounts for taker
        AssociatedTokenAccount::init_if_needed(
            accounts.taker_ata_a,
            accounts.mint_a,
            accounts.taker,
            accounts.taker,
            accounts.system_program,
            accounts.token_program,
        )?;

        // initialize neccessary mint_b ATA accounts for maker
        AssociatedTokenAccount::init_if_needed(
            accounts.maker_ata_b,
            accounts.mint_b,
            accounts.taker,
            accounts.maker,
            accounts.system_program,
            accounts.token_program,
        )?;

        // Return accounts after accounts checks and init_if_needed for the neccesary ATA's
        Ok(Self { accounts })
    }
}

impl<'a> Take<'a> {
    pub const DISCRIMINATOR: &'a u8 = &1;

    pub fn process(&mut self) -> ProgramResult {
        // Deserialize escrow
        let data = self.accounts.escrow.try_borrow_data()?;
        let escrow = Escrow::load(&data)?;

        // Check if escrow is valid
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

        let amount = TokenAccount::get_amount(self.accounts.vault)?;

        // Transfer from Vault to Taker
        Transfer {
            from: self.accounts.vault,
            to: self.accounts.taker_ata_a,
            authority: self.accounts.escrow,
            amount,
        }
        .invoke_signed(&[signer.clone()])?;

        // Close Vault
        CloseAccount {
            account: self.accounts.vault,
            destination: self.accounts.maker,
            authority: self.accounts.escrow,
        }
        .invoke_signed(&[signer.clone()])?;

        // Transfer from taker to maker
        Transfer {
            from: self.accounts.taker_ata_b,
            to: self.accounts.maker_ata_b,
            authority: self.accounts.taker,
            amount: escrow.receive,
        }
        .invoke()?;

        drop(data);
        ProgramAccount::close(self.accounts.escrow, self.accounts.taker)?;

        Ok(())
    }
}

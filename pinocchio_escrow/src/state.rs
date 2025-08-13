use core::mem::size_of;
use pinocchio::{
    program_error::ProgramError,
    pubkey::Pubkey,
};

// replicate C-like memory layout
#[repr(C)]
pub struct Escrow {
    pub seed: u64,      // Random seed for PDA derivation
    pub maker: Pubkey,  // Creator of the escrow
    pub mint_a: Pubkey, // Token being deposited
    pub mint_b: Pubkey, // Token being requested
    pub receive: u64,   // Amount of token B wanted
    pub bump: [u8; 1],  // PDA bump seed
}

impl Escrow {
    // Calculating the fixed size of the Escrow account in byytes
    pub const LEN: usize = size_of::<u64>() // seed
        + size_of::<Pubkey>()               // maker
        + size_of::<Pubkey>()               // mint_a
        + size_of::<Pubkey>()               // mint_b
        + size_of::<u64>()                  // receive
        + size_of::<[u8; 1]>();             // bump

    // Loads Escrow struct in mutable from a byte slice
    #[inline(always)]
    pub fn load_mut(bytes: &mut [u8]) -> Result<&mut Self, ProgramError> {
        // Size check
        if bytes.len() != Escrow::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        // This transmutes the input type (u8) into the output type (const Self = const Escrow)
        // bytes.as_mut_ptr() it given as input, which is a mutable pointer
        Ok(unsafe { &mut *core::mem::transmute::<*mut u8, *mut Self>(bytes.as_mut_ptr()) })
    }

    // Loads Escrow struct in immutable from a byte slice
    #[inline(always)]
    pub fn load(bytes: &[u8]) -> Result<&Self, ProgramError> {
        // Size check
        if bytes.len() != Escrow::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        // This transmutes the input type (u8) into the output type (const Self = const Escrow)
        // bytes.as_mut_ptr() it given as input, which is a immutable pointer
        Ok(unsafe { &*core::mem::transmute::<*const u8, *const Self>(bytes.as_ptr()) })
    }

    #[inline(always)]
    pub fn set_seed(&mut self, seed: u64) {
        self.seed = seed;
    }

    #[inline(always)]
    pub fn set_maker(&mut self, maker: Pubkey) {
        self.maker = maker;
    }

    #[inline(always)]
    pub fn set_mint_a(&mut self, mint_a: Pubkey) {
        self.mint_a = mint_a;
    }

    #[inline(always)]
    pub fn set_mint_b(&mut self, mint_b: Pubkey) {
        self.mint_b = mint_b;
    }

    #[inline(always)]
    pub fn set_receive(&mut self, receive: u64) {
        self.receive = receive;
    }

    #[inline(always)]
    pub fn set_bump(&mut self, bump: [u8; 1]) {
        self.bump = bump;
    }

    #[inline(always)]
    pub fn set_inner(
        &mut self,
        seed: u64,
        maker: Pubkey,
        mint_a: Pubkey,
        mint_b: Pubkey,
        receive: u64,
        bump: [u8; 1],
    ) {
        self.seed = seed;
        self.maker = maker;
        self.mint_a = mint_a;
        self.mint_b = mint_b;
        self.receive = receive;
        self.bump = bump
    }
}
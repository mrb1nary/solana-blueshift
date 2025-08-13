use pinocchio::program_error::ProgramError;

/// Custom error codes for our Pinocchio program.
#[repr(u32)]
pub enum PinocchioError {
    InvalidInstruction,     // 0
    InvalidOwner,           // 1
    UninitializedAccount,   // 2
    InvalidAccountData,     // 3
    MintCheckFailed,        // 4
    AtaCheckFailed,         // 5
    InvalidPda,             // 6
    InvalidAddress          // 7
    // Add more errors as needed...
}

// Convert our error enum into a ProgramError::Custom(code).
impl From<PinocchioError> for ProgramError {
    fn from(e: PinocchioError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
use blackgold_entropy_api::prelude::*;
use steel::*;

pub fn process_reveal(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Step 1 — Parse seed
    // Parse args.
    // This is the secret seed that the API kept hidden until sampling was complete.
    let args = Reveal::try_from_bytes(data)?;
    let seed = args.seed;

    // Step 2 — Load accounts
    // Load accounts.
    // User must sign reveal.
    let clock = Clock::get()?;
    let [signer_info, var_info] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;

    // Step 3 — Validate Var state
    let var = var_info
        .as_account_mut::<Var>(&blackgold_entropy_api::ID)?
        .assert_mut_msg(|v| clock.slot >= v.end_at, "Not ready to reveal")?
        .assert_mut_msg(|v| v.slot_hash != [0; 32], "Slot hash not sampled")?;

    
    // Step 4 — Finalize the variable
    // Finalize the variable.
    var.finalize(seed)?;

    Ok(())
}

use blackgold_entropy_api::prelude::*;
use steel::*;

pub fn process_reveal(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    //& Step 1 — Parse seed
    // Parse args.
    // This is the secret seed that the API kept hidden until sampling was complete.
    let args = Reveal::try_from_bytes(data)?;
    let seed = args.seed;

    //& Step 2 — Load accounts
    // Clock sysvar is used to validate the end_at slot.
    let clock = Clock::get()?;

    // Load the signer and variable accounts.
    // Load accounts.
    let [signer_info, var_info] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    //& Step 2b — Validate signer
    // User must sign reveal.
    signer_info.is_signer()?;

    //& Step 3 — Validate Var state
    // Load the variable account and validate its state.
    let var = var_info
        .as_account_mut::<Var>(&blackgold_entropy_api::ID)?
        .assert_mut_msg(|v| clock.slot >= v.end_at, "Not ready to reveal")?
        .assert_mut_msg(|v| v.slot_hash != [0; 32], "Slot hash not sampled")?
        .assert_mut_msg(|v| v.value == [0; 32], "Value already finalized")? // Added: Ensure the value is not already finalized
        .assert_mut_msg(|v| v.seed == [0; 32], "Seed already revealed")? // Added: Ensure the seed is not already revealed
        .assert_mut_msg(|v| v.samples > 0, "No samples remaining")?; //Added: Ensure there are samples remaining

    
    //& Step 4 — Finalize the variable
    // Finalize the variable.
    var.finalize(seed)?;

    //* Logging the reveal for transparency.
    sol_log(&format!("Var {} revealed with seed {:?}", var.id, seed));


    Ok(())
}

use blackgold_entropy_api::prelude::*;
use steel::*;

pub fn process_close(accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    // ---------------------------------------------------------
    // Step 1 — Load accounts
    // ---------------------------------------------------------
    let [signer_info, var_info, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // ---------------------------------------------------------
    // Step 2 — Validate signer
    // ---------------------------------------------------------
    //* only a signer can close the Var.
    signer_info.is_signer()?;

    // ---------------------------------------------------------
    // Step 3 — Validate Var authority
    // ---------------------------------------------------------
    //& This prevents:
    //& unauthorized closure
    //& griefing
    //& draining lamports
    //* only the Var owner can close it.
    let var = var_info
        .as_account_mut::<Var>(&blackgold_entropy_api::ID)?
        .assert_mut_msg(|v| v.authority == *signer_info.key, "Invalid var authority")?;

    // ---------------------------------------------------------
    // Optional: Prevent closing active variables
    // ---------------------------------------------------------
    /*
    assert!(
        var.seed == [0; 32] && var.slot_hash == [0; 32] && var.value == [0; 32],
        "Cannot close an active variable"
    );
    */

    // ---------------------------------------------------------
    // Optional: Prevent closing auto-mode variables
    // ---------------------------------------------------------
    /*
    assert!(
        var.is_auto == 0,
        "Auto-mode variables cannot be closed"
    );
    */

    // ---------------------------------------------------------
    // Step 4 — Validate system program
    // ---------------------------------------------------------
    //* ensures the system program account is genuine.
    system_program.is_program(&system_program::ID)?;

    // ---------------------------------------------------------
    // Step 5 — Close Var account
    // ---------------------------------------------------------
    //* lamports go to signer.
    var_info.close(signer_info)?;

    // ---------------------------------------------------------
    // Step 6 — Log event
    // ---------------------------------------------------------
    sol_log(&format!("Var {} closed by {}", var.id, signer_info.key));

    Ok(())
}

use blackgold_entropy_api::prelude::*;
use steel::*;

const MAX_ROUND_LENGTH: u64 = 500; // Maximum allowed duration for next round

//& Process the next instruction.
//& This instruction advances the variable to the next value.
//& It is called after reveal() has finalized the current round.
//& next() resets the variable's state and prepares it for the next sampling period.
pub fn process_next(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
   
    // ---------------------------------------------------------
    // Step 1 — Parse args
    // ---------------------------------------------------------
    let args = Next::try_from_bytes(data)?;
    let end_at = u64::from_le_bytes(args.end_at);

    // ---------------------------------------------------------
    // Step 2 — Load accounts
    // ---------------------------------------------------------
    let clock = Clock::get()?;
    let [signer_info, var_info] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Step 2b — Validate signer
    signer_info.is_signer()?;

    // ---------------------------------------------------------
    // Step 3 — Load and validate Var state
    // Enforce full commit–sample–reveal–finalize pipeline
    // ---------------------------------------------------------
    let var = var_info
        .as_account_mut::<Var>(&blackgold_entropy_api::ID)?
        .assert_mut_msg(|v| v.authority == *signer_info.key, "Invalid var authority")?
        .assert_mut_msg(|v| clock.slot > v.end_at, "Not ready to next")?
        .assert_mut_msg(|v| v.slot_hash != [0; 32], "Slot hash not sampled")?
        .assert_mut_msg(|v| v.seed != [0; 32], "Seed not revealed")?
        .assert_mut_msg(|v| v.value != [0; 32], "Value is not finalized")?
        .assert_mut_msg(|v| v.samples > 0, "No samples remaining")?
        .assert_mut_msg(|v| v.commit != [0; 32], "Commit missing")?;

    // ---------------------------------------------------------
    // Step 4 — Validate next round timing
    // ---------------------------------------------------------
    assert!(
        end_at > clock.slot,
        "End at must be greater than current slot"
    );

    assert!(
        end_at - clock.slot <= MAX_ROUND_LENGTH,
        "Next round too far in the future"
    );

    // ---------------------------------------------------------
    // Step 5 — Additional safety checks
    // ---------------------------------------------------------

    // Prevent seed == commit (rare but dangerous)
    assert!(
        var.seed != var.commit,
        "Seed cannot equal commit"
    );

    // Prevent calling next twice in the same slot
    assert!(
        clock.slot > var.start_at,
        "Next already called this slot"
    );

    // Auto-mode: only provider may advance
    if var.is_auto == 1 {
        assert!(
            signer_info.key == &var.provider,
            "Only provider may advance auto vars"
        );
    }

    // ---------------------------------------------------------
    // Step 6 — Update Var for next round
    // ---------------------------------------------------------

    // Seed becomes next commit (ORE-style commit chaining)
    // You may optionally harden this later.
    var.commit = var.seed;

    // Reset round state
    var.seed = [0; 32];
    var.slot_hash = [0; 32];
    var.value = [0; 32];

    // Safe decrement (no underflow)
    var.samples = var.samples.saturating_sub(1);

    // Update timing
    var.start_at = clock.slot;
    var.end_at = end_at;

    // ---------------------------------------------------------
    // Step 7 — Log event
    // ---------------------------------------------------------
    sol_log(&format!(
        "Var {} advanced to next round. Remaining samples: {}",
        var.id,
        var.samples
    ));

    Ok(())
}

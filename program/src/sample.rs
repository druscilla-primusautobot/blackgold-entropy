use blackgold_entropy_api::prelude::*;
use solana_program::{log::sol_log, slot_hashes::SlotHashes};
use steel::*;

pub fn process_sample(accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    // Load accounts.
    let clock = Clock::get()?;
    let [signer_info, var_info, slot_hashes_sysvar] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    signer_info.is_signer()?;
    
    let var = var_info
        .as_account_mut::<Var>(&blackgold_entropy_api::ID)?
        .assert_mut(|v| clock.slot >= v.end_at)?;
    slot_hashes_sysvar.is_sysvar(&sysvar::slot_hashes::ID)?;

    // Silent return.
    if var.slot_hash != [0; 32] {
        return Ok(());
    }

    // Deserialize the slot hashes.
    let slot_hashes =
        bincode::deserialize::<SlotHashes>(slot_hashes_sysvar.data.borrow().as_ref()).unwrap();

    //&OLD LOGIC
    // Record the sampled slot hash.
    // if let Some(slot_hash) = slot_hashes.get(&var.end_at) {
    //     var.slot_hash = slot_hash.to_bytes();
    //     sol_log(&format!(
    //         "Sampled hash at slot {:?}: {:?}",
    //         var.end_at,
    //         slot_hash.to_string()
    //     ));
    // } else {
    //     let hash = solana_program::keccak::hashv(&[&var.end_at.to_le_bytes()]);
    //     var.slot_hash = hash.to_bytes();
    //     sol_log(&format!(
    //         "No hash for slot {:?}. Generated: {:?}",
    //         var.end_at,
    //         hash.to_string()
    //     ));
    // }
    //&OLD LOGIC

    // Aggregate multiple slot hashes for better randomness.
    // Collect multiple slot hashes for aggregation.
    let mut buf = Vec::with_capacity(32 * 4); // 4 slots → 128 bytes total

    // Define which slots to aggregate.
    let slots = [
        var.end_at,
        var.end_at.saturating_sub(1),
        var.end_at.saturating_sub(2),
        var.end_at.saturating_sub(3),
    ];

    // For each slot, push its hash (or fallback hash) into the buffer.
    for slot in slots {
        if let Some(slot_hash) = slot_hashes.get(&slot) {
            buf.extend_from_slice(&slot_hash.to_bytes());
        } else {
            // Fallback hash if slot hash is missing.
            let fallback = solana_program::keccak::hashv(&[&slot.to_le_bytes()]);
            buf.extend_from_slice(&fallback.to_bytes());
        }
    }

    // Aggregate all slot hashes with one keccak.
    let final_hash = solana_program::keccak::hash(&buf);

    // Store the aggregated slot hash.
    var.slot_hash = final_hash.to_bytes();

    sol_log(&format!(
        "Aggregated slot hash for slots {:?}: {:?}",
        slots,
        final_hash.to_string()
    ));

    Ok(())
}

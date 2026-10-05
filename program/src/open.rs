use blackgold_entropy_api::prelude::*;
use steel::*;

//* NOTE:
// The authority is NOT the mining program ID
// Program IDs cannot sign.

// The authority IS a PDA controlled by the mining program
// This PDA signs via CPI and owns all round accounts.

// The id IS the round_id
// One entropy instance per round.

// The Var PDA uses seeds: [VAR, authority_pda, round_id]
// This is the correct architecture.

pub fn process_open(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    //* All miners participate in the same round.
    //* All miners share the same randomness value.
    //* The mining protocol uses that randomness to pick the winner.

    //* Entropy is global (for round), not per-user.

    //* Each mining round needs one fresh randomness value.
    //* That randomness:
    //* picks the winner
    //* finalizes the round
    //* resets balances
    //* advances round_id
    //* starts the next round

    //* FLOW====> open() → sample → reveal → reset → close()

    //* One Var per round.
    //* One randomness per Var.
    //* One winner per randomness.

    // Parse args.
    let clock = Clock::get()?;
    let args = Open::try_from_bytes(data)?;
    let id = u64::from_le_bytes(args.id);
    let samples = u64::from_le_bytes(args.samples);
    let end_at = u64::from_le_bytes(args.end_at);
    let is_auto = u64::from_le_bytes(args.is_auto);
    let commit = args.commit;

    // Load accounts.
    let [authority_info, payer_info, provider_info, var_info, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    //& Authority and payer must sign the transaction.
    //* The Authority = The wallet that will own the new Var account
    //* The Owner of the randomness variable (Var)
    //* This is the wallet that controls the variable we are opening.
    //* In BlackGold fork:
    //* The authority is usually the miner / user who wants randomness.
    //* They own the "Var" account (VAR ACCOUNT: PDA derived from authority + id).
    //* They approve the creation of this randomness stream.
    //* The authority is the controller, not the participant.
    authority_info.is_signer()?; //* program-derived address (PDA) or the wallet that owns the randomness variable (Var)

    //& NOTE: So authority ≠ program ID.
    //^ A Solana program ID cannot sign.
    //^ Program IDs are not accounts.
    //^ They cannot appear as is_signer.
    //^ They cannot approve PDA creation.
    //^ They cannot authorize CPI calls.
    //* mining_app_authority_pda = PDA("AUTHORITY", mining_program_id)

    //& THIS PDA - mining_app_authority_pda = PDA("AUTHORITY", mining_program_id):
    //^ is owned by out mining program
    //^ is created by our mining program
    //^ is signed via CPI (program-derived signature)
    //^ is the “identity” of the mining system
    //^ is the authority for entropy rounds

    //& Payer must sign the transaction.
    //* The Payer = The wallet funding the new Var account
    //* This is the account that pays:
    //* rent‑exemption lamports
    //* account creation cost
    //* transaction fee (if they are also the fee payer)
    //* The mining app PDA is the authority.
    //* The miner wallet is the payer.
    //* The round_id is the entropy variable ID. (payer = miner_wallet)
    payer_info.is_signer()?;

    // provider_info.is_signer()?;

    //& Validate that the variable account is empty and writable.
    var_info.is_empty()?.is_writable()?;

    //& Validate that the system program account is genuine.
    //* The System Program = Solana’s built‑in account creation program
    //* It is the program with ID: 1111....1111
    //* This program is responsible for:
    //* allocating new accounts
    //* assigning ownership
    //* transferring lamports
    //* enforcing rent rules
    system_program.is_program(&system_program::ID)?;

    // Validate the end at slot.
    //* if end is smaller than current slot (stop)
    //* if end is greater than current slor (continue)
    //* end_at = (clock.slot + 150)
    //* SLOT_DELAY = 150 slots
    //* Solana slot ≈ 400 ms
    //* 150 slots ≈ 60 seconds
    //* Enough time to guarantee unpredictability
    //* Fast enough for mining rounds - bring it from miner app as per the round requirements
    assert!(
        end_at > clock.slot,
        "End at must be greater than current slot"
    );

    // Create var account.
    //& This creates the variable account and initializes it with the provided parameters.
    //* var_info =
    //*     The PDA account that will store the Var state for this entropy round.
    //*     This account MUST be:
    //*       - derived from seeds: [VAR, authority_pubkey, round_id]
    //*       - empty before creation (no data, no owner)
    //*       - writable in the transaction
    //*     After creation, this becomes the on-chain storage for:
    //*       - commit hash
    //*       - seed (after reveal)
    //*       - slot_hash (after sample)
    //*       - final randomness value
    //*       - round timing (start_at, end_at)
    //*       - provider identity
    //*     One Var account corresponds to ONE mining round.

    //* system_program =
    //*     The real Solana system program (ID = 11111111111111111111111111111111).
    //*     Required because only the system program can:
    //*       - allocate new accounts
    //*       - assign ownership to the BlackGold Entropy program
    //*       - transfer lamports from the payer
    //*       - enforce rent-exemption rules
    //*     Passing any other account here would be a security vulnerability,
    //*     so we validate it using system_program.is_program(&system_program::ID).

    //* payer_info =
    //*     The wallet that funds the creation of the Var PDA.
    //*     This account MUST sign the transaction because lamports are deducted
    //*     from it to pay for:
    //*       - rent-exemption
    //*       - account allocation
    //*     In BlackGold, the payer is usually the mining app authority,
    //*     but we may allow flexibility (payer may be different).

    //* blackgold_entropy_api::ID =
    //*     The program ID of the BlackGold Entropy program.
    //*     This is the owner that will be assigned to the Var PDA.
    //*     After creation, only this program can modify the Var account.
    //*     This ensures the Var cannot be tampered with by external programs.

    //* Var PDA = seeds: [VAR, authority, round_id]
    //*     The deterministic address for the Var account.
    //*     Seeds:
    //*       - VAR: a static seed identifying the account type
    //*       - authority: the mining app PDA controlling the round
    //*       - round_id: unique u64 identifying the mining round
    //*     This ensures:
    //*       - one entropy instance per round
    //*       - no collisions
    //*       - predictable address for off-chain clients
    //*       - secure commit–sample–reveal lifecycle
    create_program_account::<Var>(
        var_info,
        system_program,
        payer_info,
        &blackgold_entropy_api::ID,
        &[VAR, &authority_info.key.to_bytes(), &id.to_le_bytes()],
    )?;

    // //& This sets the initial state of the variable.
    // // Initialize the variable.
    // let var = var_info.as_account_mut::<Var>(&blackgold_entropy_api::ID)?;
    // var.authority = *authority_info.key;
    // var.id = id;
    // var.provider = *provider_info.key;
    // var.commit = commit;
    // var.seed = [0; 32];
    // var.slot_hash = [0; 32];
    // var.value = [0; 32];
    // var.is_auto = is_auto;
    // var.samples = samples;
    // var.start_at = clock.slot;
    // var.end_at = end_at;

    //& Load the newly created Var account into a mutable struct.
    //& This account will store all entropy state for this mining round.
    let var = var_info.as_account_mut::<Var>(&blackgold_entropy_api::ID)?;

    //& The authority that controls this entropy variable.
    //& In BlackGold, this is the mining app authority PDA,
    //& not a user wallet. It identifies which mining system owns the round.
    var.authority = *authority_info.key;

    //& The unique identifier for this entropy variable.
    //& In BlackGold, this is the round_id. Each round has exactly one Var.
    var.id = id;

    //& The provider responsible for generating commit and seed values.
    //& This is the off-chain entropy provider (our BlackGold entropy API),
    //& whose public key is stored here for verification during reveal.
    var.provider = *provider_info.key;

    //& The initial commit hash for this round.
    //& This is generated off-chain by the entropy provider and proves
    //& that the provider has locked in a secret seed before the slothash is known.
    var.commit = commit;

    //& The seed is initially unknown. It will be revealed later.
    //& Until reveal(), the seed remains zeroed.
    var.seed = [0; 32];

    //& The slot hash sampled at end_at.
    //& This is filled during sample() and is initially zero.
    var.slot_hash = [0; 32];

    //& The final randomness value for the round.
    //& Computed during reveal() using seed + slot_hash.
    //& Starts as zero until the round is finalized.
    var.value = [0; 32];

    //& Whether the provider should automatically perform sample() and reveal()
    //& without user interaction. If true, the provider handles the full lifecycle.
    var.is_auto = is_auto;

    //& Number of values this variable will produce over its lifetime.
    //& For BlackGold mining, this is typically 1 (one randomness per round).
    var.samples = samples;

    //& The slot at which the variable was opened.
    //& Used for timing and validation of the round lifecycle.
    var.start_at = clock.slot;

    //& The future slot at which slothash must be sampled.
    //& Must be greater than the current slot. Defines when sample() is allowed.
    var.end_at = end_at;

    Ok(())
}

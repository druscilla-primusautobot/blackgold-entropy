mod close;
mod next;
mod open;
mod reveal;
mod sample;

use close::*;
use next::*;
use open::*;
use reveal::*;
use sample::*;

use blackgold_entropy_api::prelude::*;
use solana_security_txt::security_txt;
use steel::*;

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    let (ix, data) = parse_instruction(&blackgold_entropy_api::ID, program_id, data)?;

    match ix {
        // BlackgoldEntropyInstruction::Open => process_open(accounts, data)?,
        BlackgoldEntropyInstruction::Close => process_close(accounts, data)?,
        BlackgoldEntropyInstruction::Next => process_next(accounts, data)?,
        BlackgoldEntropyInstruction::Reveal => process_reveal(accounts, data)?,
        BlackgoldEntropyInstruction::Sample => process_sample(accounts, data)?,
        _ => {
            return Err(trace(
                "Invalid instruction",
                ProgramError::InvalidInstructionData,
            ))
        }
    }

    Ok(())
}

entrypoint!(process_instruction);

security_txt! {
    name: "Blackgold Entropy",
    project_url: "https://blackgold.supply",
    contacts: "email:druscilla2024@gmail.com",
    policy: "https://github.com/regolith-labs/entropy/blob/master/SECURITY.md",
    preferred_languages: "en",
    source_code: "https://github.com/regolith-labs/entropy"
    // source_revision: default_env!("GITHUB_SHA", ""),
    // source_release: default_env!("GITHUB_REF_NAME", ""),
    // auditors: "None"
}

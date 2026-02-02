#![cfg(feature = "test-sbf")]

use {
    anchor_lang::{solana_program::instruction::Instruction, InstructionData, ToAccountMetas},
    mollusk_svm::{result::Check, Mollusk},
};

#[test]
fn test_initialize() {
    let program_id = blueshift_anchor_escrow::id();

    let mollusk = Mollusk::new(&program_id, "blueshift_anchor_escrow");

    let instruction = Instruction::new_with_bytes(
        program_id,
        &blueshift_anchor_escrow::instruction::Initialize {}.data(),
        blueshift_anchor_escrow::accounts::Initialize {}.to_account_metas(None),
    );

    mollusk.process_and_validate_instruction(&instruction, &[], &[Check::success()]);
}

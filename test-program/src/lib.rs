use solana_program::{
    account_info::AccountInfo, compute_units::sol_remaining_compute_units,
    entrypoint::ProgramResult, log::sol_log, pubkey::Pubkey,
};

solana_program::entrypoint!(process_instruction);

const FIXED_CPI_COST: u64 = 1000;
const REMAINING_CU_COST: u64 = 100;

// A simple solana program that transfers 1 lamport twice
fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    _data: &[u8],
) -> ProgramResult {
    let original_balance = accounts[0].lamports();
    // Send from account zero to account one, thrice.
    // 1) First with standard invoke.
    // 2) Then with our invoke
    // 3) Then with our invoke_unchecked
    let transfer =
        solana_program::system_instruction::transfer(accounts[0].key, accounts[1].key, 1);

    // 1) First with standard invoke_signed.
    sol_log("invoking system program via solana_program::program::invoke");
    let first = sol_remaining_compute_units();
    solana_program::program::invoke(&transfer, &accounts[..2])?;
    let second = sol_remaining_compute_units();
    assert_eq!(accounts[0].lamports(), original_balance - 1);
    sol_log(&format!(
        "invoked system program via solana_program::program::invoke successfully: {} cus",
        first - second - FIXED_CPI_COST - REMAINING_CU_COST
    ));

    // 2) Then with our invoke_signed
    sol_log("invoking system program via our invoke");
    let first = sol_remaining_compute_units();
    solana_invoke::invoke(&transfer, &accounts[..2])?;
    let second = sol_remaining_compute_units();
    assert_eq!(accounts[0].lamports(), original_balance - 2);
    sol_log(&format!(
        "invoked system program via our invoke successfully: {} cus",
        first - second - FIXED_CPI_COST - REMAINING_CU_COST,
    ));

    // 3) Then with our invoke_unchecked
    sol_log("invoking system program via our invoke");
    let first = sol_remaining_compute_units();
    solana_invoke::invoke_unchecked(&transfer, &accounts[..2])?;
    let second = sol_remaining_compute_units();
    assert_eq!(accounts[0].lamports(), original_balance - 3);
    sol_log(&format!(
        "invoked system program via our invoke_unchecked successfully: {} cus",
        first - second - FIXED_CPI_COST - REMAINING_CU_COST,
    ));

    Ok(())
}

#[cfg(test)]
mod tests {
    use solana_program_test::ProgramTest;
    use solana_sdk::{
        account::AccountSharedData,
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        signature::Keypair,
        signer::Signer,
        system_program,
        transaction::Transaction,
    };

    #[tokio::test]
    async fn test_cpi() {
        // Initialize program test with this program
        let program_id = Pubkey::new_unique();
        let mut program_test = ProgramTest::default();
        program_test.prefer_bpf(true);
        program_test.add_program("triple_transfer", program_id, None);
        let mut ctx = program_test.start_with_context().await;

        // Initialize two accounts: sender and receiver
        let sender = Keypair::new();
        let receiver = Pubkey::new_unique();

        // Fund sender and receiver (needs enough for rent)
        ctx.set_account(
            &sender.pubkey(),
            &AccountSharedData::new(1_000_000_000, 0, &system_program::ID),
        );
        ctx.set_account(
            &receiver,
            &AccountSharedData::new(1_000_000_000, 0, &system_program::ID),
        );

        // Build and sign invoke transaction
        let invoke_instruction = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(sender.pubkey(), true),
                AccountMeta::new(receiver, false),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
            data: vec![],
        };
        let blockhash = ctx.get_new_latest_blockhash().await.unwrap();
        let invoke_transaction =
            Transaction::new_signed_with_payer(&[invoke_instruction], None, &[&sender], blockhash);

        // Execute
        ctx.banks_client
            .process_transaction(invoke_transaction)
            .await
            .unwrap();
    }
}

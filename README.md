# `solana-invoke`

A drop-in replacement for `solana_program::program::invoke*` with better compute and heap efficiency

## Summary

The current CPI functions `solana_program::program::invoke*` perform unnecessary copies and allocations. This crate removes these inefficiencies in a manner that is 100% backwards compatible.

The compute and heap savings scale with the amount of accounts and data passed in on CPI. Even in the test program featured in `test-program/`, which passes in only two accounts and O(16 bytes) of data, a significant saving is observed (overhead reduced from 536 cus -> 197 cus).

```rust
// test-program schematic. logs and asserts are redacted.

// A simple solana program that transfers 1 lamport thrice
fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    _data: &[u8],
) -> ProgramResult {
    // Send from account zero to account one, thrice.
    // 1) First with standard invoke.
    // 2) Then with our invoke
    // 3) Then with our invoke_unchecked
    let transfer =
        solana_program::system_instruction::transfer(accounts[0].key, accounts[1].key, 1);

    // 1) First with standard invoke_signed.
    solana_program::program::invoke(&transfer, &accounts[..2])?;

    // 2) Then with our invoke_signed
    solana_invoke::invoke(&transfer, &accounts[..2])?;

    // 3) Then with our invoke_unchecked
    solana_invoke::invoke_unchecked(&transfer, &accounts[..2])?;

    Ok(())
}
```

Output:

```text
Program 1111111QLbz7JHiBTspS962RLKV8GndWFwiEaqKM invoke [1]
Program log: invoking system program via solana_program::program::invoke
Program 11111111111111111111111111111111 invoke [2]
Program 11111111111111111111111111111111 success
Program log: invoked system program via solana_program::program::invoke successfully: 536 cus
Program log: invoking system program via our invoke
Program 11111111111111111111111111111111 invoke [2]
Program 11111111111111111111111111111111 success
Program log: invoked system program via our invoke successfully: 392 cus
Program log: invoking system program via our invoke
Program 11111111111111111111111111111111 invoke [2]
Program 11111111111111111111111111111111 success
Program log: invoked system program via our invoke successfully: 197 cus
Program 1111111QLbz7JHiBTspS962RLKV8GndWFwiEaqKM consumed 7864 of 200000 compute units
Program 1111111QLbz7JHiBTspS962RLKV8GndWFwiEaqKM success
```

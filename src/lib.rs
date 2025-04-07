#![cfg_attr(not(test), warn(unused_crate_dependencies))]

// constants
const STANDARD_TOKEN_COST: u8 = 4;
const TOTAL_COST_FLOOR_PER_TOKEN: u8 = 10;
const INITCODE_WORST_CODE: u8 = 2;

/// Compute the gas used by a transaction.
fn compute_gas_used(calldata: &[u8]) -> u64 {

}
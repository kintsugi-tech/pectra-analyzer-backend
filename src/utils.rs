use revm::interpreter::gas::get_tokens_in_calldata;

/// The cost of the calldata floor per token.
///
/// This is taken from the EIP-7623 spec.
const TOTAL_COST_FLOOR_PER_TOKEN: u64 = 10;
/// The base stipend for the calldata.
pub const BASE_STIPEND: u64 = 21000;
/// The size of a blob in bytes.
pub const BLOB_SIZE: u64 = 128_000;
/// The standard cost of calldata token.
pub const STANDARD_TOKEN_COST: u64 = 4;
/// It returns the gas cost of the calldata following the new EIP-7623 rules.
///
/// Link: https://eips.ethereum.org/EIPS/eip-7623
pub fn compute_calldata_gas(calldata: &[u8]) -> u64 {
    let tokens_in_calldata = get_tokens_in_calldata(calldata, true); // TODO: check if is_istanbul spec id is correct
    TOTAL_COST_FLOOR_PER_TOKEN * tokens_in_calldata
}

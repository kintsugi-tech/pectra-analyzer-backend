use revm::interpreter::gas::get_tokens_in_calldata;

/// The cost of the calldata floor per token.
///
/// This is taken from the EIP-7623 spec.
const TOTAL_COST_FLOOR_PER_TOKEN: u64 = 10;
/// The block number of the istanbul hard fork on Ethereum mainnet.
const ISTANBUL_BLOCK_NUMBER: u64 = 9_069_000;
/// The base stipend for the calldata.
pub const BASE_STIPEND: u64 = 21000;
/// The size of a blob in bytes.
pub const BLOB_SIZE: u64 = 128_000;
/// The standard cost of calldata token.
pub const STANDARD_TOKEN_COST: u64 = 4;
/// The number of bytes in a blob.
pub const BYTES_PER_BLOB: u64 = 131_072;
/// Is istabul hard fork enabled?
const fn is_istanbul_enabled(block_number: u64) -> bool {
    block_number >= ISTANBUL_BLOCK_NUMBER
}
/// It returns the gas cost of the calldata following the new EIP-7623 rules.
///
/// Link: https://eips.ethereum.org/EIPS/eip-7623
pub fn compute_calldata_gas(calldata: &[u8], block_number: u64) -> u64 {
    let is_istanbul = is_istanbul_enabled(block_number);
    let tokens_in_calldata = get_tokens_in_calldata(calldata, is_istanbul);
    TOTAL_COST_FLOOR_PER_TOKEN * tokens_in_calldata
}
/// It returns the gas cost of the calldata following legacy rules.
///
/// Link: https://eips.ethereum.org/EIPS/eip-7623
pub fn compute_legacy_calldata_gas(calldata: &[u8], block_number: u64) -> u64 {
    let is_istanbul = is_istanbul_enabled(block_number);
    let tokens_in_calldata = get_tokens_in_calldata(calldata, is_istanbul);
    STANDARD_TOKEN_COST * tokens_in_calldata
}

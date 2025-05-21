# Pectralizer

A powerful Ethereum transaction and contract analysis tool that provides detailed gas usage analysis, including support for EIP-4844 (blob transactions) and EIP-7623 (calldata gas cost changes).

## Features

- 🔍 Transaction analysis with detailed gas usage breakdown
- 📊 Contract analysis with historical transaction data
- 🌐 Support for EIP-4844 blob transactions
- ⛽ EIP-7623 calldata gas cost analysis
- 🚀 Fast and efficient API endpoints
- 🐳 Docker support for easy deployment

## Prerequisites

- Rust 1.76 or later
- Docker (optional)
- Ethereum provider URL (e.g., Infura)
- Etherscan API key

## Environment Variables

Create a `.env` file in the project root with the following variables:

```env
# Required
ETHEREUM_PROVIDER=your_ethereum_provider_url
ETHERSCAN_API_KEY=your_etherscan_api_key
CHAIN_ID=1 (right now Ethereum mainnet and Sepolia are supported)

# Optional
PORT=3000  # Default: 3000
RUST_LOG=info  # Default: info
```

## Running with Docker

1. Build and run the container:

```bash
docker-compose up --build
```

2. Or specify a custom port:

```bash
PORT=8080 docker-compose up
```

## API Endpoints

### GET /

Returns a welcome message.

### GET /tx

Analyzes a transaction by its hash.

Query Parameters:

- `tx_hash`: The transaction hash to analyze

Example:

```bash
curl "http://localhost:3000/tx?tx_hash=0xf9b3708d3c8a07f7c26bbd336c2746977787b126fbc95e2df816a74d599957c4"
```

Response:

```json
{
  "gas_used": 21000,
  "gas_price": 5767832048,
  "blob_gas_price": 2793617096,
  "blob_gas_used": 393216,
  "eip_7623_calldata_gas": 15574830,
  "legacy_calldata_gas": 6229932
}
```

### GET /contract

Analyzes a contract's transactions.

Query Parameters:

- `contract_address`: The contract address to analyze

Example:

```bash
curl "http://localhost:3000/contract?contract_address=0x41dDf7fC14a579E0F3f2D698e14c76d9d486B9F7"
```

## Development

1. Clone the repository:

```bash
git clone https://github.com/yourusername/pectralizer.git
cd pectralizer
```

2. Install dependencies:

```bash
cargo build
```

3. Run the development server:

```bash
cargo run
```

## Testing

Run the test suite:

```bash
cargo test
```

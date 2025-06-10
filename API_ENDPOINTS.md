# L2 Batches Analytics API Endpoints

This document describes the new API endpoints added for L2 batch analysis.

## Existing Endpoints

- `GET /` - Welcome message
- `GET /tx?tx_hash=<hash>` - Single transaction analysis
- `GET /contract?contract_address=<address>` - Contract analysis

## New L2 Analytics Endpoints

### 1. Daily Transactions per Batcher

**Endpoint:** `GET /daily_txs`

**Parameters:**
- `batcher_address` (string) - The batcher address to filter by
- `start_timestamp` (i64) - Start timestamp (Unix timestamp)
- `end_timestamp` (i64) - End timestamp (Unix timestamp)

**Example:**
```
GET /daily_txs?batcher_address=0x5050F69a9786F081509234F1a7F4684b5E5b76C9&start_timestamp=1640995200&end_timestamp=1641081600
```

**Response:**
```json
{
  "batcher_address": "0x5050F69a9786F081509234F1a7F4684b5E5b76C9",
  "tx_count": 42
}
```

### 2. ETH Saved

**Endpoint:** `GET /eth_saved`

**Parameters:**
- `batcher_address` (string) - The batcher address to filter by
- `start_timestamp` (i64) - Start timestamp (Unix timestamp)
- `end_timestamp` (i64) - End timestamp (Unix timestamp)

**Example:**
```
GET /eth_saved?batcher_address=0x5050F69a9786F081509234F1a7F4684b5E5b76C9&start_timestamp=1640995200&end_timestamp=1641081600
```

**Response:**
```json
{
  "batcher_address": "0x5050F69a9786F081509234F1a7F4684b5E5b76C9",
  "total_eth_saved_wei": "1234567890123456789"
}
```

**Note:** ETH saved is calculated as the difference between the cost with EIP-7623 calldata and the actual cost with blob data.

### 3. Total Blob Data Gas

**Endpoint:** `GET /blob_data_gas`

**Parameters:**
- `batcher_address` (string) - The batcher address to filter by
- `start_timestamp` (i64) - Start timestamp (Unix timestamp)
- `end_timestamp` (i64) - End timestamp (Unix timestamp)

**Example:**
```
GET /blob_data_gas?batcher_address=0x5050F69a9786F081509234F1a7F4684b5E5b76C9&start_timestamp=1640995200&end_timestamp=1641081600
```

**Response:**
```json
{
  "batcher_address": "0x5050F69a9786F081509234F1a7F4684b5E5b76C9",
  "total_blob_data_gas": 1234567890
}
```

### 4. Total Pectra Data Gas (EIP-7623)

**Endpoint:** `GET /pectra_data_gas`

**Parameters:**
- `batcher_address` (string) - The batcher address to filter by
- `start_timestamp` (i64) - Start timestamp (Unix timestamp)
- `end_timestamp` (i64) - End timestamp (Unix timestamp)

**Example:**
```
GET /pectra_data_gas?batcher_address=0x5050F69a9786F081509234F1a7F4684b5E5b76C9&start_timestamp=1640995200&end_timestamp=1641081600
```

**Response:**
```json
{
  "batcher_address": "0x5050F69a9786F081509234F1a7F4684b5E5b76C9",
  "total_pectra_data_gas": 9876543210
}
```

## Aggregated L2 Analytics Endpoints (All Batchers)

The following endpoints return data for all batchers at once, eliminating the need to make separate API calls for each batcher address.

### 1. All Daily Transactions

**Endpoint:** `GET /all_daily_txs`

**Parameters:**
- `start_timestamp` (i64) - Start timestamp (Unix timestamp)
- `end_timestamp` (i64) - End timestamp (Unix timestamp)

**Example:**
```
GET /all_daily_txs?start_timestamp=1640995200&end_timestamp=1641081600
```

**Response:**
```json
{
  "batchers": [
    {
      "batcher_address": "0x5050F69a9786F081509234F1a7F4684b5E5b76C9",
      "tx_count": 42
    },
    {
      "batcher_address": "0x6887246668a3b87F54DeB3b94Ba47a6f63F32985",
      "tx_count": 38
    }
  ]
}
```

### 2. All ETH Saved

**Endpoint:** `GET /all_eth_saved`

**Parameters:**
- `start_timestamp` (i64) - Start timestamp (Unix timestamp)
- `end_timestamp` (i64) - End timestamp (Unix timestamp)

**Example:**
```
GET /all_eth_saved?start_timestamp=1640995200&end_timestamp=1641081600
```

**Response:**
```json
{
  "batchers": [
    {
      "batcher_address": "0x5050F69a9786F081509234F1a7F4684b5E5b76C9",
      "total_eth_saved_wei": "1234567890123456789"
    },
    {
      "batcher_address": "0x6887246668a3b87F54DeB3b94Ba47a6f63F32985",
      "total_eth_saved_wei": "987654321098765432"
    }
  ]
}
```

### 3. All Blob Data Gas

**Endpoint:** `GET /all_blob_data_gas`

**Parameters:**
- `start_timestamp` (i64) - Start timestamp (Unix timestamp)
- `end_timestamp` (i64) - End timestamp (Unix timestamp)

**Example:**
```
GET /all_blob_data_gas?start_timestamp=1640995200&end_timestamp=1641081600
```

**Response:**
```json
{
  "batchers": [
    {
      "batcher_address": "0x5050F69a9786F081509234F1a7F4684b5E5b76C9",
      "total_blob_data_gas": 1234567890
    },
    {
      "batcher_address": "0x6887246668a3b87F54DeB3b94Ba47a6f63F32985",
      "total_blob_data_gas": 876543210
    }
  ]
}
```

### 4. All Pectra Data Gas (EIP-7623)

**Endpoint:** `GET /all_pectra_data_gas`

**Parameters:**
- `start_timestamp` (i64) - Start timestamp (Unix timestamp)
- `end_timestamp` (i64) - End timestamp (Unix timestamp)

**Example:**
```
GET /all_pectra_data_gas?start_timestamp=1640995200&end_timestamp=1641081600
```

**Response:**
```json
{
  "batchers": [
    {
      "batcher_address": "0x5050F69a9786F081509234F1a7F4684b5E5b76C9",
      "total_pectra_data_gas": 9876543210
    },
    {
      "batcher_address": "0x6887246668a3b87F54DeB3b94Ba47a6f63F32985",
      "total_pectra_data_gas": 5432109876
    }
  ]
}
```

### 5. Seven-Day Stats (Daily Snapshots)

**Endpoint:** `GET /seven_day_stats`

**Parameters:** none

**Description:**
Returns, for each monitored batcher, the most recent 7 daily snapshot rows produced by the background snapshot loop. Each field is returned as an array whose order corresponds to the `timestamps` array.

**Response:**
```json
{
  "batchers": [
    {
      "batcher_address": "0x5050F69a9786F081509234F1a7F4684b5E5b76C9",
      "timestamps":            [1759430400, 1759516800, 1759603200, 1759689600, 1759776000, 1759862400, 1759948800],
      "total_daily_txs":       [221, 270, 390, 130, 532, 421, 1009],
      "total_eth_saved_wei":   ["123", "456", "789", "..."],
      "total_blob_data_gas":   [100, 200, 300, 400, 500, 600, 700],
      "total_pectra_data_gas": [150, 250, 350, 450, 550, 650, 750]
    },
    { "batcher_address": "0x6887246668a3b87F54DeB3b94Ba47a6f63F32985", "timestamps": [...], ... }
  ]
}
```

## Technical Notes

- All timestamps are in Unix timestamp format (seconds since January 1, 1970)
- Gas values are in gas units
- ETH values are in wei (1 ETH = 10^18 wei)
- The endpoints query the SQLite database created by the L2 monitoring service
- The database is automatically populated by the `run_l2_batches_monitoring_service`
- **Individual endpoints** require a `batcher_address` parameter to filter results for a specific batcher
- **Aggregated endpoints** (`/all_*`) return data for all batchers and only require timestamp parameters
- Aggregated endpoints are more efficient when you need data for multiple batchers at once

## Monitored Batcher Addresses

Currently the system monitors the following batcher addresses:
- `0x5050F69a9786F081509234F1a7F4684b5E5b76C9` (Base)
- `0x6887246668a3b87F54DeB3b94Ba47a6f63F32985` (Optimism)

## Server Startup

To start the server with the new APIs:

```bash
# Set required environment variables
export ETHEREUM_PROVIDER="your_ethereum_rpc_url"
export ETHERSCAN_API_KEY="your_etherscan_api_key"
export CHAIN_ID="1"  # 1 for mainnet, 11155111 for sepolia
export PORT="3000"   # optional, default 3000

# Start the server
cargo run
```

The server will start both the HTTP API and the L2 monitoring service in parallel. 
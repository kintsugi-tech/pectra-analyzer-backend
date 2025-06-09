# L2 Batches Analytics API Endpoints

Questo documento descrive i nuovi endpoint API aggiunti per l'analisi dei batch L2.

## Endpoint Esistenti

- `GET /` - Messaggio di benvenuto
- `GET /tx?tx_hash=<hash>` - Analisi di una singola transazione
- `GET /contract?contract_address=<address>` - Analisi di un contratto

## Nuovi Endpoint L2 Analytics

### 1. Transazioni Giornaliere per Batcher

**Endpoint:** `GET /daily_txs`

**Parametri:**
- `start_timestamp` (i64) - Timestamp di inizio (Unix timestamp)
- `end_timestamp` (i64) - Timestamp di fine (Unix timestamp)

**Esempio:**
```
GET /daily_txs?start_timestamp=1640995200&end_timestamp=1641081600
```

**Risposta:**
```json
{
  "batcher_txs": [
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

### 2. ETH Risparmiato

**Endpoint:** `GET /eth_saved`

**Parametri:**
- `start_timestamp` (i64) - Timestamp di inizio (Unix timestamp)
- `end_timestamp` (i64) - Timestamp di fine (Unix timestamp)

**Esempio:**
```
GET /eth_saved?start_timestamp=1640995200&end_timestamp=1641081600
```

**Risposta:**
```json
{
  "total_eth_saved_wei": "1234567890123456789",
  "transaction_savings": [
    {
      "tx_hash": "0xabc123...",
      "eth_saved_wei": "123456789012345",
      "timestamp": 1640995300
    }
  ]
}
```

**Nota:** L'ETH risparmiato è calcolato come la differenza tra il costo che si avrebbe con EIP-7623 calldata e il costo effettivo con blob data.

### 3. Gas Totale Blob Data

**Endpoint:** `GET /blob_data_gas`

**Parametri:**
- `start_timestamp` (i64) - Timestamp di inizio (Unix timestamp)
- `end_timestamp` (i64) - Timestamp di fine (Unix timestamp)

**Esempio:**
```
GET /blob_data_gas?start_timestamp=1640995200&end_timestamp=1641081600
```

**Risposta:**
```json
{
  "total_blob_data_gas": 1234567890
}
```

### 4. Gas Totale Pectra Data (EIP-7623)

**Endpoint:** `GET /pectra_data_gas`

**Parametri:**
- `start_timestamp` (i64) - Timestamp di inizio (Unix timestamp)
- `end_timestamp` (i64) - Timestamp di fine (Unix timestamp)

**Esempio:**
```
GET /pectra_data_gas?start_timestamp=1640995200&end_timestamp=1641081600
```

**Risposta:**
```json
{
  "total_pectra_data_gas": 9876543210
}
```

## Note Tecniche

- Tutti i timestamp sono in formato Unix timestamp (secondi dal 1 gennaio 1970)
- I valori di gas sono in unità di gas
- I valori ETH sono in wei (1 ETH = 10^18 wei)
- Gli endpoint interrogano il database SQLite creato dal servizio di monitoring L2
- Il database viene popolato automaticamente dal servizio `run_l2_batches_monitoring_service`

## Indirizzi Batcher Monitorati

Attualmente il sistema monitora i seguenti indirizzi batcher:
- `0x5050F69a9786F081509234F1a7F4684b5E5b76C9` (Base)
- `0x6887246668a3b87F54DeB3b94Ba47a6f63F32985` (Optimism)

## Avvio del Server

Per avviare il server con le nuove API:

```bash
# Imposta le variabili d'ambiente necessarie
export ETHEREUM_PROVIDER="your_ethereum_rpc_url"
export ETHERSCAN_API_KEY="your_etherscan_api_key"
export CHAIN_ID="1"  # 1 per mainnet, 11155111 per sepolia
export PORT="3000"   # opzionale, default 3000

# Avvia il server
cargo run
```

Il server avvierà sia l'API HTTP che il servizio di monitoring L2 in parallelo. 
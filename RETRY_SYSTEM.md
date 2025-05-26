# Failed Transaction Retry System

## Overview

The L2 monitor now includes a robust retry system for handling transactions that fail during analysis. Instead of losing failed transactions, they are saved to a retry queue and processed later with exponential backoff.

## How It Works

### 1. Failed Transaction Detection
When the `analyze_transaction` function fails (e.g., due to blobscan API errors), instead of skipping the transaction:
- The transaction is saved to a `failed_transactions` table
- Error details, timestamps, and retry count are recorded
- The transaction is scheduled for retry with exponential backoff

### 2. Retry Processing
A parallel `RetryHandler` service runs alongside the main monitor:
- Checks for failed transactions ready for retry every 30 seconds
- Processes transactions using exponential backoff (1min, 2min, 4min, 8min, 16min, max 1hour)
- Maximum of 5 retry attempts before giving up
- Successfully processed transactions are moved to the main database

### 3. Database Schema

#### Main Table: `l2_batches_txs`
- Stores successfully processed transactions
- Same schema as before

#### New Table: `failed_transactions`
```sql
CREATE TABLE failed_transactions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tx_hash TEXT NOT NULL UNIQUE,
    batcher_address TEXT NOT NULL,
    error_message TEXT NOT NULL,
    retry_count INTEGER NOT NULL DEFAULT 0,
    next_retry_at INTEGER NOT NULL,
    first_failed_at INTEGER NOT NULL,
    last_attempted_at INTEGER NOT NULL
);
```

## Benefits

1. **No Data Loss**: Failed transactions are preserved and retried
2. **Resilient to API Issues**: Temporary blobscan API failures don't cause permanent data loss
3. **Exponential Backoff**: Prevents overwhelming external APIs during outages
4. **Automatic Recovery**: System automatically recovers when APIs come back online
5. **Monitoring**: Clear logging of retry attempts and failures

## Configuration

### Retry Parameters (in `retry_handler.rs`)
- `MAX_RETRY_ATTEMPTS`: 5 attempts
- `BASE_RETRY_DELAY`: 60 seconds (1 minute)
- `MAX_RETRY_DELAY`: 3600 seconds (1 hour)
- Retry check interval: 30 seconds

### Exponential Backoff Schedule
- Attempt 1: 1 minute delay
- Attempt 2: 2 minutes delay  
- Attempt 3: 4 minutes delay
- Attempt 4: 8 minutes delay
- Attempt 5: 16 minutes delay
- After 5 failures: Transaction is removed from queue

## Monitoring

The system provides detailed logging:
- When transactions are added to retry queue
- Retry attempt progress
- Successful recoveries
- Final failures after max attempts

## Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   L2 Monitor    │    │  Failed TX Queue │    │  Retry Handler  │
│                 │    │                  │    │                 │
│ ┌─────────────┐ │    │ ┌──────────────┐ │    │ ┌─────────────┐ │
│ │ New TX      │ │───▶│ │ Failed TX    │ │◄───│ │ Retry Logic │ │
│ │ Processing  │ │    │ │ Storage      │ │    │ │             │ │
│ └─────────────┘ │    │ └──────────────┘ │    │ └─────────────┘ │
│                 │    │                  │    │                 │
│ ┌─────────────┐ │    │                  │    │ ┌─────────────┐ │
│ │ Success     │ │───▶│                  │    │ │ Success     │ │
│ │ → Main DB   │ │    │                  │    │ │ → Main DB   │ │
│ └─────────────┘ │    │                  │    │ └─────────────┘ │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

## Usage

The retry system is automatically enabled when the L2 monitor starts. No additional configuration is required. The system will:

1. Continue normal transaction processing
2. Automatically handle failures by queuing for retry
3. Process retries in the background
4. Provide detailed logging of all operations

This ensures maximum reliability and data integrity for the L2 batch monitoring system. 
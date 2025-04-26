use axum::{Router, routing::get};
use pectralizer::server::handlers::{root_handler, tx_handler};

#[tokio::main]
async fn main() {
    // load .env environment variables
    dotenv::dotenv().ok();
    // build the application
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/tx", get(tx_handler));
    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[cfg(test)]
mod tests {
    use alloy_consensus::{Transaction, Typed2718};
    use alloy_primitives::{FixedBytes, hex::FromHex};
    use alloy_provider::{Provider, ProviderBuilder};
    use axum::extract::Query;
    use pectralizer::{
        server::{handlers::tx_handler, types::TxHashQuery},
        utils::{BASE_STIPEND, BLOB_SIZE, STANDARD_TOKEN_COST, compute_calldata_gas},
    };
    use std::env;
    use url::Url;

    #[tokio::test]
    async fn test_tx_handler() {
        let query = TxHashQuery {
            tx_hash: "0xf9b3708d3c8a07f7c26bbd336c2746977787b126fbc95e2df816a74d599957c4"
                .to_string(),
        };
        let response = tx_handler(Query(query)).await;
        println!("response: {:?}", response.0);
    }

    #[tokio::test]
    async fn test_provider() {
        // ethereum infura endpoint
        let ethereum_infura_url = env::var("ETHEREUM_PROVIDER").unwrap();
        // create provider
        let provider = ProviderBuilder::new().on_http(Url::parse(&ethereum_infura_url).unwrap());
        // transform tx hash into a fixed bytes
        let tx_hash = "0xf9b3708d3c8a07f7c26bbd336c2746977787b126fbc95e2df816a74d599957c4";
        let tx_hash_bytes = FixedBytes::from_hex(&tx_hash).unwrap();
        // get tx
        let tx = provider
            .get_transaction_by_hash(tx_hash_bytes)
            .await
            .unwrap()
            .unwrap();
        // get calldata
        let calldata = tx.input();
        // get receipt
        let receipt = provider
            .get_transaction_receipt(tx_hash_bytes)
            .await
            .unwrap()
            .unwrap();
        // get total gas used
        let gas_used = receipt.gas_used;
        // compute EIP-7623 calldata gas
        let eip_7623_calldata_gas = compute_calldata_gas(calldata) + BASE_STIPEND;
        println!("gas_used: {}", gas_used);
        println!("eip_7623_calldata_gas: {}", eip_7623_calldata_gas);
        let is_blob = tx.is_eip4844();
        println!("is_blob: {}", is_blob);
        if is_blob {
            let blob_gas_used = tx.blob_gas_used().unwrap();
            println!("blob_gas_used: {}", blob_gas_used);
            let blob_data = tx.blob_versioned_hashes().unwrap();
            // get blob data
            // then call `compute_calldata_gas` with the blob data
            // and at this point we can return calldata cost vs blob cost
        }
    }
}

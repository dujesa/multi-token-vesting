mod db;
mod decoder;
mod processor;

use std::sync::Arc;

use carbon_core::{
    error::CarbonResult,
    pipeline::{Pipeline, ShutdownStrategy},
};
use carbon_log_metrics::LogMetrics;
use carbon_rpc_block_crawler_datasource::RpcBlockCrawler;
use carbon_rpc_block_subscribe_datasource::{
    Filters as BlockSubscribeFilters, RpcBlockSubscribe,
};
use decoder::VestingDecoder;
use processor::VestingProcessor;
use solana_client::rpc_config::RpcBlockConfig;
use solana_transaction_status::{
    TransactionDetails, UiTransactionEncoding,
};

#[tokio::main]
async fn main() -> CarbonResult<()> {
    dotenvy::dotenv().ok();
    env_logger::init();

    let rpc_url = std::env::var("RPC_URL").expect("RPC_URL must be set");
    let ws_url = std::env::var("WS_URL").expect("WS_URL must be set");
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let start_slot: u64 = std::env::var("START_SLOT")
        .expect("START_SLOT must be set")
        .parse()
        .expect("START_SLOT must be a valid u64");

    // Database
    let pool = db::init_pool(&database_url).await;
    db::run_migrations(&pool).await;

    log::info!("starting vesting indexer from slot {start_slot}");

    // Historical backfill
    let block_crawler = RpcBlockCrawler::new(
        rpc_url,
        start_slot,
        None,
        None,
        RpcBlockConfig {
            encoding: Some(UiTransactionEncoding::Binary),
            transaction_details: Some(TransactionDetails::Full),
            max_supported_transaction_version: Some(0),
            ..Default::default()
        },
        Some(5),
        Some(10),
    );

    // Live subscription
    let block_subscribe = RpcBlockSubscribe::new(
        ws_url,
        BlockSubscribeFilters {
            block_filter: solana_client::rpc_config::RpcBlockSubscribeFilter::MentionsAccountOrProgram(
                decoder::PROGRAM_ID.to_string(),
            ),
            block_subscribe_config: Some(solana_client::rpc_config::RpcBlockSubscribeConfig {
                commitment: Some(solana_client::rpc_config::CommitmentConfig::confirmed()),
                encoding: Some(UiTransactionEncoding::Binary),
                transaction_details: Some(TransactionDetails::Full),
                max_supported_transaction_version: Some(0),
                show_rewards: Some(false),
            }),
        },
    );

    Pipeline::builder()
        .datasource(block_crawler)
        .datasource(block_subscribe)
        .instruction(VestingDecoder, VestingProcessor { pool: pool.clone() })
        .metrics(Arc::new(LogMetrics::new()))
        .shutdown_strategy(ShutdownStrategy::ProcessPending)
        .build()?
        .run()
        .await?;

    Ok(())
}

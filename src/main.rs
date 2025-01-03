use dotenv::dotenv;
use eyre::Result;
use slog::{error, info, warn};
use std::sync::Arc;
use tokio::{self};

mod context;
mod repository;
mod solidity;
mod worker;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv()?;

    let app_context = Arc::new(context::context()?);
    info!(app_context.logger, "Main function is starting...");

    let block_subscription_worker_handle = {
        let config = Arc::clone(&app_context);
        async move {
            worker::solidity::run_block_listener_subscription_worker(&config).await?;
            Ok(())
        }
    };

    let order_filler_worker_handle = {
        let config = Arc::clone(&app_context);
        async move {
            if let Err(e) = worker::filler::run_order_filler_worker(&config).await {
                return Err(e);
            }
            Ok(())
        }
    };

    let result = tokio::select! {
        res = order_filler_worker_handle => ("order_filler_worker", res),
        res = block_subscription_worker_handle => ("event_subscription_worker", res),
    };

    match result {
        (worker_name, Ok(_)) => {
            warn!(app_context.logger, "{} has terminated unexpectedly", worker_name);
            Ok(())
        },
        (worker_name, Err(e)) => {
            error!(
                app_context.logger,
                "{} encountered an error: {:?}", worker_name, e
            );
            Err(e.into())
        },
    }
}

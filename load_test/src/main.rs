use anyhow::Result;
use clap::Parser;

mod cli;
mod config;
mod activities;
mod metrics;
mod scenarios;
mod workflows;
mod utils;
mod worker;

use cli::{Cli, Mode, Scenario};

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let cli = Cli::parse();

    match cli.mode {
        Mode::Worker(args) => {
            // Initialize tracing
            let subscriber = tracing_subscriber::fmt()
                .with_max_level(if args.verbose {
                    tracing::Level::DEBUG
                } else {
                    tracing::Level::INFO
                })
                .finish();
            tracing::subscriber::set_global_default(subscriber)?;

            // Run worker (blocks until Ctrl+C)
            worker::run_worker(
                args.endpoint,
                args.domain,
                args.task_list,
                args.worker_profile,
                args.verbose,
            )
            .await?;
        }

        Mode::Client(client_args) => {
            // Initialize tracing
            let subscriber = tracing_subscriber::fmt()
                .with_max_level(if client_args.verbose {
                    tracing::Level::DEBUG
                } else {
                    tracing::Level::INFO
                })
                .finish();
            tracing::subscriber::set_global_default(subscriber)?;

            tracing::info!("Cadence Load Test Client Starting...");
            tracing::info!("Endpoint: {}", client_args.endpoint);
            tracing::info!("Domain: {}", client_args.domain);
            tracing::info!("Task List: {}", client_args.task_list);
            tracing::info!("Duration: {}s", client_args.duration);
            tracing::warn!(
                "NOTE: Ensure a worker is running on domain='{}' task_list='{}'",
                client_args.domain,
                client_args.task_list
            );

            // Run the selected scenario
            match client_args.scenario.clone() {
                Scenario::HighThroughput(args) => {
                    tracing::info!("Running High Throughput scenario");
                    tracing::info!("  Target Rate: {}/sec", args.target_rate);
                    tracing::info!("  Workflow Type: {}", args.workflow_type);
                    tracing::info!("  Warmup: {}s", args.warmup);
                    scenarios::high_throughput::run(client_args, args).await?;
                }
                Scenario::HighConcurrency(args) => {
                    tracing::info!("Running High Concurrency scenario");
                    tracing::info!("  Target Count: {}", args.target_count);
                    tracing::info!("  Workflow Duration: {}s", args.workflow_duration);
                    scenarios::high_concurrency::run(client_args, args).await?;
                }
                Scenario::SustainedLoad(args) => {
                    tracing::info!("Running Sustained Load scenario");
                    tracing::info!("  Rate: {}/sec", args.rate);
                    tracing::info!("  Workflow Type: {}", args.workflow_type);
                    scenarios::sustained_load::run(client_args, args).await?;
                }
                Scenario::Spike(args) => {
                    tracing::info!("Running Spike/Burst scenario");
                    tracing::info!("  Baseline Rate: {}/sec", args.baseline_rate);
                    tracing::info!("  Spike Rate: {}/sec", args.spike_rate);
                    tracing::info!("  Spike Duration: {}s", args.spike_duration);
                    tracing::info!("  Spike Interval: {}s", args.spike_interval);
                    scenarios::spike_burst::run(client_args, args).await?;
                }
                Scenario::Failure(args) => {
                    tracing::info!("Running Failure scenario");
                    tracing::info!("  Rate: {}/sec", args.rate);
                    tracing::info!("  Failure Rate: {}", args.failure_rate);
                    tracing::info!("  Max Retries: {}", args.max_retries);
                    scenarios::failure::run(client_args, args).await?;
                }
            }

            tracing::info!("Load test complete");
        }
    }

    Ok(())
}

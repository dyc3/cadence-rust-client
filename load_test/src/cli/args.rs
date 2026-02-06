use clap::{Args, Parser, Subcommand};

/// Cadence Load Testing Tool
#[derive(Parser, Debug)]
#[command(name = "load-test")]
#[command(about = "Load testing tool for Cadence worker capacity testing")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub mode: Mode,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Mode {
    /// Start a Cadence worker (runs until Ctrl+C)
    Worker(WorkerArgs),

    /// Run load test client (sends workflow execution requests)
    Client(ClientArgs),
}

#[derive(Args, Debug, Clone)]
pub struct WorkerArgs {
    /// Cadence gRPC endpoint
    #[arg(
        long,
        default_value = "http://localhost:7833",
        env = "CADENCE_ENDPOINT"
    )]
    pub endpoint: String,

    /// Domain name
    #[arg(long, default_value = "loadtest-domain", env = "CADENCE_DOMAIN")]
    pub domain: String,

    /// Task list name
    #[arg(long, default_value = "loadtest-tasks", env = "CADENCE_TASK_LIST")]
    pub task_list: String,

    /// Worker configuration profile: dev, high-perf, stress
    #[arg(long, default_value = "high-perf")]
    pub worker_profile: String,

    /// Enable verbose logging
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Args, Debug, Clone)]
pub struct ClientArgs {
    /// Cadence gRPC endpoint
    #[arg(
        long,
        default_value = "http://localhost:7833",
        env = "CADENCE_ENDPOINT"
    )]
    pub endpoint: String,

    /// Domain name
    #[arg(long, default_value = "loadtest-domain", env = "CADENCE_DOMAIN")]
    pub domain: String,

    /// Task list name
    #[arg(long, default_value = "loadtest-tasks", env = "CADENCE_TASK_LIST")]
    pub task_list: String,

    /// Test duration in seconds
    #[arg(long, default_value = "60")]
    pub duration: u64,

    /// Metrics reporting interval in seconds
    #[arg(long, default_value = "5")]
    pub report_interval: u64,

    /// Enable verbose logging
    #[arg(short, long)]
    pub verbose: bool,

    #[command(subcommand)]
    pub scenario: Scenario,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Scenario {
    /// Test maximum workflows/second worker can handle
    HighThroughput(HighThroughputArgs),

    /// Test many simultaneous in-flight workflows
    HighConcurrency(HighConcurrencyArgs),

    /// Test consistent load over extended time to detect degradation
    SustainedLoad(SustainedLoadArgs),

    /// Test worker behavior under sudden load increases
    Spike(SpikeArgs),

    /// Test error handling and retry logic
    Failure(FailureArgs),
}

#[derive(Args, Debug, Clone)]
pub struct HighThroughputArgs {
    /// Target workflows per second
    #[arg(long)]
    pub target_rate: f64,

    /// Workflow type to use: noop_workflow, cpu_bound_workflow, io_bound_workflow, failing_workflow
    #[arg(
        long,
        default_value = "noop_workflow",
        value_parser = ["noop_workflow", "cpu_bound_workflow", "io_bound_workflow", "failing_workflow"]
    )]
    pub workflow_type: String,

    /// Warmup duration in seconds (ramp up from 0 to target rate)
    #[arg(long, default_value = "10")]
    pub warmup: u64,

    /// CPU iterations (used by cpu_bound_workflow)
    #[arg(long, default_value = "10000")]
    pub cpu_iterations: usize,

    /// IO delay in milliseconds (used by io_bound_workflow)
    #[arg(long, default_value = "100")]
    pub io_delay_ms: u64,

    /// Failure rate 0.0-1.0 (used by failing_workflow)
    #[arg(long, default_value = "0.0")]
    pub failure_rate: f64,

    /// Max retry attempts (used by failing_workflow)
    #[arg(long, default_value = "3")]
    pub max_retries: usize,
}

#[derive(Args, Debug, Clone)]
pub struct HighConcurrencyArgs {
    /// Target concurrent workflow count
    #[arg(long)]
    pub target_count: usize,

    /// Workflow duration in seconds
    #[arg(long, default_value = "30")]
    pub workflow_duration: u64,
}

#[derive(Args, Debug, Clone)]
pub struct SustainedLoadArgs {
    /// Workflows per second to maintain
    #[arg(long)]
    pub rate: f64,

    /// Workflow type to use: noop_workflow, cpu_bound_workflow, io_bound_workflow, failing_workflow
    #[arg(
        long,
        default_value = "noop_workflow",
        value_parser = ["noop_workflow", "cpu_bound_workflow", "io_bound_workflow", "failing_workflow"]
    )]
    pub workflow_type: String,

    /// CPU iterations (used by cpu_bound_workflow)
    #[arg(long, default_value = "10000")]
    pub cpu_iterations: usize,

    /// IO delay in milliseconds (used by io_bound_workflow)
    #[arg(long, default_value = "100")]
    pub io_delay_ms: u64,

    /// Failure rate 0.0-1.0 (used by failing_workflow)
    #[arg(long, default_value = "0.0")]
    pub failure_rate: f64,

    /// Max retry attempts (used by failing_workflow)
    #[arg(long, default_value = "3")]
    pub max_retries: usize,
}

#[derive(Args, Debug, Clone)]
pub struct SpikeArgs {
    /// Baseline workflows per second
    #[arg(long)]
    pub baseline_rate: f64,

    /// Spike workflows per second
    #[arg(long)]
    pub spike_rate: f64,

    /// Duration of each spike in seconds
    #[arg(long, default_value = "10")]
    pub spike_duration: u64,

    /// Interval between spikes in seconds
    #[arg(long, default_value = "30")]
    pub spike_interval: u64,
}

#[derive(Args, Debug, Clone)]
pub struct FailureArgs {
    /// Workflows per second
    #[arg(long)]
    pub rate: f64,

    /// Failure rate as decimal (0.0-1.0)
    #[arg(long)]
    pub failure_rate: f64,

    /// Max retry attempts
    #[arg(long, default_value = "3")]
    pub max_retries: usize,
}

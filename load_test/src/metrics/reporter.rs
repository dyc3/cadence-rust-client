//! Console reporter for metrics with real-time updates

use super::collector::MetricsCollector;
use std::io::{self, Write};
use tokio::time::{interval, Duration};

/// Start periodic metrics reporting (every N seconds)
pub async fn start_periodic_reporter(collector: MetricsCollector, interval_secs: u64) {
    let mut ticker = interval(Duration::from_secs(interval_secs));

    loop {
        ticker.tick().await;
        
        // Update system metrics before printing
        collector.update_system_metrics();
        
        print_live_metrics(&collector);
    }
}

/// Print live metrics (clears screen and updates in place)
pub fn print_live_metrics(collector: &MetricsCollector) {
    // Clear screen and move cursor to top
    print!("\x1B[2J\x1B[1;1H");
    
    let metrics = collector.get_snapshot();
    let elapsed = collector.elapsed_seconds();
    let wf_latency = collector.get_workflow_latency_percentiles();
    let act_latency = collector.get_activity_latency_percentiles();

    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║             Cadence Load Test - Live Metrics                  ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    
    // Time elapsed
    println!("\n⏱️  Elapsed Time: {:02}:{:02}:{:02}", 
        elapsed / 3600, (elapsed % 3600) / 60, elapsed % 60);

    // Workflows
    println!("\n┌─ WORKFLOWS ─────────────────────────────────────────────────┐");
    println!("│  Started:      {:>8}    In-Flight:  {:>8}              │",
        metrics.workflow.started, metrics.workflow.in_flight);
    println!("│  Completed:    {:>8}    Failed:     {:>8}              │",
        metrics.workflow.completed, metrics.workflow.failed);
    
    if metrics.workflow.started > 0 {
        let success_rate = (metrics.workflow.completed as f64 / metrics.workflow.started as f64) * 100.0;
        let throughput = if elapsed > 0 {
            metrics.workflow.completed as f64 / elapsed as f64
        } else {
            0.0
        };
        println!("│  Success Rate: {:>7.2}%    Throughput: {:>7.2}/sec        │",
            success_rate, throughput);
    }
    println!("└─────────────────────────────────────────────────────────────┘");

    // Workflow Latencies
    if wf_latency.count > 0 {
        println!("\n┌─ WORKFLOW LATENCY (ms) ─────────────────────────────────────┐");
        println!("│  Min: {:>6}  P50: {:>6}  P95: {:>6}  P99: {:>6}  Max: {:>6}│",
            wf_latency.min, wf_latency.p50, wf_latency.p95, wf_latency.p99, wf_latency.max);
        println!("│  Mean: {:>8.2} ms    Count: {:>10}                    │",
            wf_latency.mean, wf_latency.count);
        println!("└─────────────────────────────────────────────────────────────┘");
    }

    // Activities
    if metrics.activity.started > 0 {
        println!("\n┌─ ACTIVITIES ────────────────────────────────────────────────┐");
        println!("│  Started:      {:>8}    In-Flight:  {:>8}              │",
            metrics.activity.started, metrics.activity.in_flight);
        println!("│  Completed:    {:>8}    Failed:     {:>8}              │",
            metrics.activity.completed, metrics.activity.failed);
        
        if metrics.activity.started > 0 {
            let success_rate = (metrics.activity.completed as f64 / metrics.activity.started as f64) * 100.0;
            println!("│  Success Rate: {:>7.2}%                                  │", success_rate);
        }
        println!("└─────────────────────────────────────────────────────────────┘");

        // Activity Latencies
        if act_latency.count > 0 {
            println!("\n┌─ ACTIVITY LATENCY (ms) ─────────────────────────────────────┐");
            println!("│  Min: {:>6}  P50: {:>6}  P95: {:>6}  P99: {:>6}  Max: {:>6}│",
                act_latency.min, act_latency.p50, act_latency.p95, act_latency.p99, act_latency.max);
            println!("│  Mean: {:>8.2} ms    Count: {:>10}                    │",
                act_latency.mean, act_latency.count);
            println!("└─────────────────────────────────────────────────────────────┘");
        }
    }

    // System metrics
    println!("\n┌─ SYSTEM ────────────────────────────────────────────────────┐");
    println!("│  CPU Usage:    {:>6.1}%    Memory: {:>6} / {:>6} MB       │",
        metrics.system.cpu_usage, metrics.system.memory_used_mb, metrics.system.memory_total_mb);
    println!("└─────────────────────────────────────────────────────────────┘");

    println!("\n  [Press Ctrl+C to stop test]");
    
    // Flush stdout to ensure immediate display
    let _ = io::stdout().flush();
}

/// Print final summary report
pub fn print_final_report(collector: &MetricsCollector) {
    let metrics = collector.get_snapshot();
    let elapsed = collector.elapsed_seconds();
    let wf_latency = collector.get_workflow_latency_percentiles();
    let act_latency = collector.get_activity_latency_percentiles();

    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║                    FINAL TEST REPORT                           ║");
    println!("╚════════════════════════════════════════════════════════════════╝");

    println!("\n📊 WORKFLOWS");
    println!("   Total Started:        {:>10}", metrics.workflow.started);
    println!("   Total Completed:      {:>10}", metrics.workflow.completed);
    println!("   Total Failed:         {:>10}", metrics.workflow.failed);

    if elapsed > 0 {
        let throughput = metrics.workflow.completed as f64 / elapsed as f64;
        println!("   Throughput:           {:>10.2} workflows/sec", throughput);
    }

    if metrics.workflow.started > 0 {
        let success_rate = (metrics.workflow.completed as f64 / metrics.workflow.started as f64) * 100.0;
        println!("   Success Rate:         {:>10.2}%", success_rate);
    }

    if wf_latency.count > 0 {
        println!("\n📈 WORKFLOW LATENCY");
        println!("   Min:                  {:>10} ms", wf_latency.min);
        println!("   P50 (Median):         {:>10} ms", wf_latency.p50);
        println!("   P95:                  {:>10} ms", wf_latency.p95);
        println!("   P99:                  {:>10} ms", wf_latency.p99);
        println!("   Max:                  {:>10} ms", wf_latency.max);
        println!("   Mean:                 {:>10.2} ms", wf_latency.mean);
    }

    if metrics.activity.started > 0 {
        println!("\n⚙️  ACTIVITIES");
        println!("   Total Started:        {:>10}", metrics.activity.started);
        println!("   Total Completed:      {:>10}", metrics.activity.completed);
        println!("   Total Failed:         {:>10}", metrics.activity.failed);

        if metrics.activity.started > 0 {
            let success_rate = (metrics.activity.completed as f64 / metrics.activity.started as f64) * 100.0;
            println!("   Success Rate:         {:>10.2}%", success_rate);
        }

        if act_latency.count > 0 {
            println!("\n📈 ACTIVITY LATENCY");
            println!("   Min:                  {:>10} ms", act_latency.min);
            println!("   P50 (Median):         {:>10} ms", act_latency.p50);
            println!("   P95:                  {:>10} ms", act_latency.p95);
            println!("   P99:                  {:>10} ms", act_latency.p99);
            println!("   Max:                  {:>10} ms", act_latency.max);
            println!("   Mean:                 {:>10.2} ms", act_latency.mean);
        }
    }

    println!("\n⏱️  Test Duration: {:.2} seconds", elapsed);
    println!("════════════════════════════════════════════════════════════════\n");
}

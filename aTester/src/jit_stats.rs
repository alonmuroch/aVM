use std::collections::BTreeSet;

use crate::TestReport;

pub fn print_jit_stats(reports: &[TestReport]) {
    let mut thresholds = BTreeSet::new();
    let mut tracked_pcs: u64 = 0;
    let mut total_hits: u64 = 0;
    let mut hot_pcs: u64 = 0;
    let mut cache_entries: u64 = 0;
    let mut failed_entries: u64 = 0;
    let mut compile_attempts: u64 = 0;
    let mut compile_successes: u64 = 0;
    let mut compile_failures: u64 = 0;
    let mut jit_execs: u64 = 0;
    let mut cache_hits: u64 = 0;
    let mut enabled_count = 0;

    for report in reports {
        let Some(stats) = report.jit_stats.as_ref() else {
            continue;
        };
        if !stats.enabled {
            continue;
        }
        enabled_count += 1;
        thresholds.insert(stats.hot_threshold);
        tracked_pcs = tracked_pcs.saturating_add(stats.tracked_pcs as u64);
        total_hits = total_hits.saturating_add(stats.total_hits);
        hot_pcs = hot_pcs.saturating_add(stats.hot_pcs as u64);
        cache_entries = cache_entries.saturating_add(stats.cache_entries as u64);
        failed_entries = failed_entries.saturating_add(stats.failed_entries as u64);
        compile_attempts = compile_attempts.saturating_add(stats.compile_attempts);
        compile_successes = compile_successes.saturating_add(stats.compile_successes);
        compile_failures = compile_failures.saturating_add(stats.compile_failures);
        jit_execs = jit_execs.saturating_add(stats.jit_execs);
        cache_hits = cache_hits.saturating_add(stats.cache_hits);
    }

    if enabled_count == 0 {
        return;
    }

    let hot_threshold = format_thresholds(&thresholds);
    println!("\n--- JIT stats ---");
    println!("enabled: {enabled_count}/{} tests", reports.len());
    println!("hot_threshold: {hot_threshold} (interpreter hits before compile)");
    println!(
        "tracked_pcs: {} (unique PCs with hit counts)",
        format_u64(tracked_pcs)
    );
    println!(
        "total_hits: {} (sum of per-PC hit counts)",
        format_u64(total_hits)
    );
    println!(
        "hot_pcs: {} (PCs at or above hot threshold)",
        format_u64(hot_pcs)
    );
    println!(
        "compile_attempts: {} (success {}, failed {})",
        format_u64(compile_attempts),
        format_u64(compile_successes),
        format_u64(compile_failures)
    );
    println!(
        "cache_entries: {} (compiled traces available)",
        format_u64(cache_entries)
    );
    println!(
        "cache_hits: {} (executed cached trace)",
        format_u64(cache_hits)
    );
    println!(
        "jit_execs: {} (total compiled trace runs)",
        format_u64(jit_execs)
    );
    println!(
        "failed_entries: {} (PCs marked unsupported)",
        format_u64(failed_entries)
    );
}

fn format_u64(value: u64) -> String {
    format_number(value.to_string())
}

fn format_number(mut digits: String) -> String {
    let mut out = String::new();
    let mut count = 0;
    while let Some(ch) = digits.pop() {
        if count == 3 {
            out.push(',');
            count = 0;
        }
        out.push(ch);
        count += 1;
    }
    out.chars().rev().collect()
}

fn format_thresholds(thresholds: &BTreeSet<u32>) -> String {
    let mut out = String::new();
    for (idx, value) in thresholds.iter().enumerate() {
        if idx > 0 {
            out.push_str(", ");
        }
        out.push_str(&value.to_string());
    }
    out
}

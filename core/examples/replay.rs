//! Replays a recorded hand trace at several lookaheads (#44):
//! `cargo run --release --example replay -- trace.txt [lookahead ms ...]`

use air_core::trace::{parse, score};

/// Frame to speaker on the Oppo with prediction off: capture→result ~100 ms plus
/// audio out ~28 ms (readout, 2026-11-22). Felt ≈ this minus the lead.
const PIPELINE_MS: f64 = 128.0;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: replay <trace.txt> [lookahead ms ...]");
    let mut lookaheads: Vec<u64> = args.map(|a| a.parse().expect("lookahead in ms")).collect();
    if lookaheads.is_empty() {
        lookaheads = vec![0, 40, 60, 80, 100, 120];
    }
    let text = std::fs::read_to_string(&path).expect("read trace");
    let trace = parse(&text).unwrap_or_else(|e| panic!("{path}: {e}"));
    let secs = trace.frames.last().map_or(0.0, |f| {
        (f.timestamp_ns - trace.frames[0].timestamp_ns) as f64 / 1e9
    });
    println!("{path}: {} frames, {secs:.0} s", trace.frames.len());
    println!("lookahead  crossings  missed  FALSE  predicted  lead avg/min/max ms  felt est ms  error avg ms");
    for ms in lookaheads {
        let s = score(&trace, ms * 1_000_000);
        let n = s.leads_ms.len().max(1) as f64;
        let lead = s.leads_ms.iter().sum::<f64>() / n;
        let min = s.leads_ms.iter().copied().fold(f64::INFINITY, f64::min);
        let max = s.leads_ms.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let err = s.errors_ms.iter().sum::<f64>() / s.errors_ms.len().max(1) as f64;
        let mut sorted = s.leads_ms.clone();
        sorted.sort_by(f64::total_cmp);
        let q = |f: f64| sorted.get((f * sorted.len() as f64) as usize).copied().unwrap_or(0.0);
        eprintln!("         lead quartiles {:.0} / {:.0} / {:.0} ms", q(0.25), q(0.5), q(0.75));
        println!(
            "{ms:>6} ms  {:>9}  {:>6}  {:>5}  {:>9}  {lead:>6.0} {min:>5.0} {max:>5.0}     {:>8.0}     {err:>8.1}",
            s.crossings,
            s.missed,
            s.false_plucks,
            s.errors_ms.len(),
            PIPELINE_MS - lead,
        );
    }
}

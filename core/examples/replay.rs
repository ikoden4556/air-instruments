//! Replays a recorded hand trace at several lookaheads (#44):
//! `cargo run --release --example replay -- trace.txt [lookahead ms ...]`

use air_core::trace::{parse, score};

/// Frame to speaker on the Oppo with prediction off: capture→result ~100 ms plus
/// audio out ~28 ms (readout, 2026-11-22). Felt ≈ this minus the lead.
const PIPELINE_MS: f64 = 128.0;

/// For each unmatched predicted Pluck: the nearer Plucker's signed distance to its String
/// (screen widths, + on the side it came from) from 4 frames before it fired to 8 after.
fn dump_false(trace: &air_core::trace::Trace, predicted: &[air_core::trace::Predicted]) {
    use air_core::engine::Engine;
    use air_core::pitch::Key;
    let mut engine = Engine::new(trace.aspect, Key { root: 0 });
    let pluckers: Vec<_> = trace
        .frames
        .iter()
        .map(|f| {
            engine.update(f);
            engine.pluckers()
        })
        .collect();
    for p in predicted.iter().filter(|p| !p.real) {
        let Some(at) = trace
            .frames
            .iter()
            .position(|f| f.timestamp_ns == p.frame_ns)
        else {
            continue;
        };
        let (a, b) = (p.string.left, p.string.right);
        let len = (b.x - a.x).hypot((b.y - a.y) * trace.aspect);
        let side = |q: air_core::frame::Landmark| {
            ((b.x - a.x) * (q.y - a.y) * trace.aspect - (b.y - a.y) * trace.aspect * (q.x - a.x))
                / len
        };
        let hand = (0..2)
            .min_by(|&i, &j| {
                let d = |k: usize| pluckers[at][k].map_or(f32::MAX, |q| side(q).abs());
                d(i).total_cmp(&d(j))
            })
            .unwrap();
        let sign = pluckers[at][hand].map_or(1.0, |q| side(q).signum());
        let series: Vec<String> = (at.saturating_sub(4)..(at + 9).min(pluckers.len()))
            .map(|k| {
                let mark = if k == at { "*" } else { "" };
                pluckers[k][hand].map_or("  --".into(), |q| format!("{mark}{:.3}", side(q) * sign))
            })
            .collect();
        println!(
            "  FALSE at {:.2} s, ahead {:.0} ms, {:.1} w/s: {}",
            p.frame_ns as f64 / 1e9 - trace.frames[0].timestamp_ns as f64 / 1e9,
            p.ahead_ms,
            p.velocity * 5.0,
            series.join(" ")
        );
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .expect("usage: replay <trace.txt> [lookahead ms ...]");
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
    println!(
        "lookahead  crossings  missed  FALSE  predicted  lead avg/min/max ms  felt est ms  error avg ms"
    );
    for ms in lookaheads {
        let s = score(&trace, ms * 1_000_000);
        let n = s.leads_ms.len().max(1) as f64;
        let lead = s.leads_ms.iter().sum::<f64>() / n;
        let min = s.leads_ms.iter().copied().fold(f64::INFINITY, f64::min);
        let max = s.leads_ms.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let err = s.errors_ms.iter().sum::<f64>() / s.errors_ms.len().max(1) as f64;
        let mut sorted = s.leads_ms.clone();
        sorted.sort_by(f64::total_cmp);
        let q = |f: f64| {
            sorted
                .get((f * sorted.len() as f64) as usize)
                .copied()
                .unwrap_or(0.0)
        };
        eprintln!(
            "         lead quartiles {:.0} / {:.0} / {:.0} ms",
            q(0.25),
            q(0.5),
            q(0.75)
        );
        println!(
            "{ms:>6} ms  {:>9}  {:>6}  {:>5}  {:>9}  {lead:>6.0} {min:>5.0} {max:>5.0}     {:>8.0}     {err:>8.1}",
            s.crossings,
            s.missed,
            s.false_plucks,
            s.errors_ms.len(),
            PIPELINE_MS - lead,
        );
        if std::env::var_os("DETAIL").is_some() {
            // Predicted Plucks by speed and by how far ahead they predicted: real/unmatched.
            let speeds = [0.0, 1.0, 2.0, 3.0, 4.0, 5.1];
            println!("  ahead ms  | speed widths/s: 0-1, 1-2, 2-3, 3-4, 4-5+  (real/unmatched)");
            for ahead in (0..ms.max(20)).step_by(20) {
                let cells: Vec<String> = speeds
                    .windows(2)
                    .map(|w| {
                        let cell = s.predicted.iter().filter(|p| {
                            (ahead as f64..ahead as f64 + 20.0).contains(&p.ahead_ms)
                                && (w[0]..w[1]).contains(&(p.velocity * 5.0))
                        });
                        let (good, bad) =
                            cell.fold(
                                (0, 0),
                                |(g, b), p| {
                                    if p.real { (g + 1, b) } else { (g, b + 1) }
                                },
                            );
                        format!("{good:>3}/{bad:<3}")
                    })
                    .collect();
                println!("  {ahead:>3}-{:<4}   {}", ahead + 20, cells.join("  "));
            }
        }
        if std::env::var_os("DUMP").is_some() {
            dump_false(&trace, &s.predicted);
        }
    }
}

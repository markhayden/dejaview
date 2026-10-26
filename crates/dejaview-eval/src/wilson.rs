//! The 95% Wilson score interval for a binomial proportion (SPEC "Metrics").

/// The two-sided 95% normal quantile.
pub const Z_95: f64 = 1.959_963_984_540_054;

/// The Wilson interval for `successes` of `trials`; `None` when there are no trials.
pub fn wilson(successes: u64, trials: u64, z: f64) -> Option<(f64, f64)> {
    if trials == 0 || successes > trials {
        return None;
    }
    let n = trials as f64;
    let p = successes as f64 / n;
    let z2 = z * z;
    let denominator = 1.0 + z2 / n;
    let centre = (p + z2 / (2.0 * n)) / denominator;
    let half = z * ((p * (1.0 - p) / n) + z2 / (4.0 * n * n)).sqrt() / denominator;
    Some(((centre - half).max(0.0), (centre + half).min(1.0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stats_wilson_matches_the_reference_table() {
        // Reference values computed independently (Python, double precision) from the closed form
        // at z = 1.959963984540054.
        let table: &[(u64, u64, f64, f64)] = &[
            (0, 10, 0.0, 0.2775327998628892),
            (10, 10, 0.7224672001371107, 0.9999999999999999),
            (7, 10, 0.39677814746114537, 0.8922087325936989),
            (1, 2, 0.09453120573423074, 0.9054687942657693),
            (50, 100, 0.4038315303659956, 0.5961684696340044),
            (3, 20, 0.052368745896216595, 0.36041886474075696),
        ];
        for &(s, n, lo, hi) in table {
            let (l, h) = wilson(s, n, Z_95).expect("trials");
            assert!((l - lo).abs() < 1e-9, "{s}/{n} lower {l} vs {lo}");
            assert!((h - hi).abs() < 1e-9, "{s}/{n} upper {h} vs {hi}");
        }
        assert_eq!(wilson(0, 0, Z_95), None);
        assert_eq!(wilson(3, 2, Z_95), None);
    }
}

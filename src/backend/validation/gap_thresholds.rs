/// Gap Threshold System - Liquidity-Based Rules
///
/// Core insight: Different pool sizes require different gap minimums
/// Smaller pools = higher gap needed (riskier execution)
/// Larger pools = smaller gap acceptable (more reliable execution)
///
/// Gap thresholds by liquidity:
/// - 30k-50k:      15% gap minimum
/// - 50k-100k:     10% gap minimum
/// - 100k-300k:    8% gap minimum
/// - 300k-1M:      6% gap minimum
/// - 1M+:          5% gap minimum
///
/// Rationale:
/// - Small pools: High slippage risk, need big gaps to profit
/// - Large pools: Low slippage risk, smaller gaps are profitable
/// - Scales with reality of pool execution

use log::{info, warn};

/// Gap threshold entry (pool size range + minimum gap required)
#[derive(Debug, Clone)]
pub struct GapThreshold {
    pub min_liquidity: u64,
    pub max_liquidity: u64,
    pub min_gap_bps: u64, // basis points (1 bps = 0.01%)
    pub description: &'static str,
}

/// Get gap thresholds in order
pub fn get_gap_thresholds() -> Vec<GapThreshold> {
    vec![
        GapThreshold {
            min_liquidity: 1_000_000,
            max_liquidity: u64::MAX,
            min_gap_bps: 500, // 5%
            description: "1M+ liquidity → 5% gap minimum",
        },
        GapThreshold {
            min_liquidity: 300_000,
            max_liquidity: 1_000_000,
            min_gap_bps: 600, // 6%
            description: "300k-1M liquidity → 6% gap minimum",
        },
        GapThreshold {
            min_liquidity: 100_000,
            max_liquidity: 300_000,
            min_gap_bps: 800, // 8%
            description: "100k-300k liquidity → 8% gap minimum",
        },
        GapThreshold {
            min_liquidity: 50_000,
            max_liquidity: 100_000,
            min_gap_bps: 1000, // 10%
            description: "50k-100k liquidity → 10% gap minimum",
        },
        GapThreshold {
            min_liquidity: 30_000,
            max_liquidity: 50_000,
            min_gap_bps: 1500, // 15%
            description: "30k-50k liquidity → 15% gap minimum",
        },
    ]
}

/// Calculate minimum required gap for given liquidity
pub fn get_min_gap_for_liquidity(liquidity: u64) -> Option<u64> {
    for threshold in get_gap_thresholds() {
        if liquidity >= threshold.min_liquidity && liquidity < threshold.max_liquidity {
            return Some(threshold.min_gap_bps);
        }
    }
    None
}

/// Check if gap meets liquidity requirement
pub fn is_gap_sufficient(liquidity: u64, gap_bps: u64) -> Result<bool, String> {
    match get_min_gap_for_liquidity(liquidity) {
        Some(min_gap) => {
            let sufficient = gap_bps >= min_gap;
            if sufficient {
                info!(
                    "✅ Gap sufficient: {} bps ≥ {} bps (liquidity: {})",
                    gap_bps, min_gap, liquidity
                );
            } else {
                warn!(
                    "❌ Gap insufficient: {} bps < {} bps (liquidity: {})",
                    gap_bps, min_gap, liquidity
                );
            }
            Ok(sufficient)
        }
        None => Err(format!("Liquidity {} outside valid range (30k-∞)", liquidity)),
    }
}

/// Get description of why trade meets/fails gap requirement
pub fn explain_gap_requirement(liquidity: u64, gap_bps: u64) -> String {
    match get_min_gap_for_liquidity(liquidity) {
        Some(min_gap) => {
            let threshold = get_gap_thresholds()
                .into_iter()
                .find(|t| liquidity >= t.min_liquidity && liquidity < t.max_liquidity)
                .map(|t| t.description)
                .unwrap_or("Unknown");

            if gap_bps >= min_gap {
                format!(
                    "✅ {} - Gap {} bps ≥ required {} bps",
                    threshold, gap_bps, min_gap
                )
            } else {
                format!(
                    "❌ {} - Gap {} bps < required {} bps (need +{} bps)",
                    threshold,
                    gap_bps,
                    min_gap,
                    min_gap - gap_bps
                )
            }
        }
        None => format!("❌ Liquidity {} outside valid range", liquidity),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_pool_high_gap_required() {
        let liquidity = 40_000; // 30k-50k range
        let gap = 1500; // 15%

        assert_eq!(get_min_gap_for_liquidity(liquidity), Some(1500));
        assert!(is_gap_sufficient(liquidity, gap).unwrap());
        assert!(!is_gap_sufficient(liquidity, 1400).unwrap());
    }

    #[test]
    fn test_medium_pool_medium_gap() {
        let liquidity = 150_000; // 100k-300k range
        let gap = 800; // 8%

        assert_eq!(get_min_gap_for_liquidity(liquidity), Some(800));
        assert!(is_gap_sufficient(liquidity, gap).unwrap());
        assert!(!is_gap_sufficient(liquidity, 700).unwrap());
    }

    #[test]
    fn test_large_pool_low_gap() {
        let liquidity = 5_000_000; // 1M+ range
        let gap = 500; // 5%

        assert_eq!(get_min_gap_for_liquidity(liquidity), Some(500));
        assert!(is_gap_sufficient(liquidity, gap).unwrap());
        assert!(!is_gap_sufficient(liquidity, 400).unwrap());
    }

    #[test]
    fn test_gap_progression() {
        // As liquidity increases, gap requirement decreases
        assert_eq!(get_min_gap_for_liquidity(40_000), Some(1500)); // 15%
        assert_eq!(get_min_gap_for_liquidity(75_000), Some(1000)); // 10%
        assert_eq!(get_min_gap_for_liquidity(200_000), Some(800)); // 8%
        assert_eq!(get_min_gap_for_liquidity(500_000), Some(600)); // 6%
        assert_eq!(get_min_gap_for_liquidity(2_000_000), Some(500)); // 5%
    }

    #[test]
    fn test_gap_explanation() {
        let explanation = explain_gap_requirement(40_000, 1500);
        assert!(explanation.contains("✅"));
        assert!(explanation.contains("15%"));

        let explanation = explain_gap_requirement(40_000, 1000);
        assert!(explanation.contains("❌"));
        assert!(explanation.contains("500")); // needs 500 more bps
    }
}

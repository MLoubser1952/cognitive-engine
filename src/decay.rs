//! Hybrid Decay Model (SHO-103)
//!
//! Implements biologically-accurate memory decay based on neuroscience research.
//!
//! # The Problem with Pure Exponential Decay
//!
//! Traditional memory systems use exponential decay: `w(t) = w₀ × e^(-λt)`
//!
//! This produces a "cliff" effect where memories drop rapidly and then flatten:
//! - Day 1: 100% → 95%
//! - Day 7: 95% → 70%
//! - Day 30: 70% → 15% (steep cliff)
//!
//! # The Solution: Hybrid Decay
//!
//! Human memory follows a power-law for long-term retention, not exponential.
//!
//! This module implements a hybrid model:
//! - **Consolidation phase** (t < 3 days): Exponential decay
//!   - Fast filtering of noise and weak associations
//!   - Matches short-term synaptic depression
//! - **Long-term phase** (t ≥ 3 days): Power-law decay
//!   - Heavy tail preserves important memories longer
//!   - Matches empirical human forgetting curves
//!
//! ```text
//!         Exponential              Power-Law
//!         (consolidation)          (long-term retention)
//!
//! Strength │ ╲
//!     100% │  ╲
//!          │   ╲
//!      60% │    ╲___
//!          │        ╲____
//!      30% │             ╲________
//!          │                      ╲___________
//!       5% │─────────────────────────────────────────
//!          └────┬────────┬─────────────────────────► Time
//!               │        │
//!            t_cross   (days)
//!          (3 days)
//! ```
//!
//! # References
//!
//! - Wixted & Ebbesen (1991) "On the Form of Forgetting"
//! - Wixted (2004) "The psychology and neuroscience of forgetting"
//! - Anderson & Schooler (1991) "Reflections of the Environment in Memory"

use crate::constants::{
    DECAY_CROSSOVER_DAYS, DECAY_LAMBDA_CONSOLIDATION, POWERLAW_BETA, POWERLAW_BETA_POTENTIATED,
};
use crate::graph_memory::EdgeCategory;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Calculates the hybrid decay factor for a given elapsed time.
///
/// Returns a value between 0.0 and 1.0 representing the retention ratio.
///
/// # Arguments
///
/// * `days_elapsed` - Time since last activation in days
/// * `potentiated` - Whether this is a potentiated/important memory (uses slower decay)
///
/// # Returns
///
/// Decay factor to multiply with original strength: `new_strength = old_strength * decay_factor`
///
/// # Example
///
/// ```ignore
/// let factor = hybrid_decay_factor(7.0, false);
/// let new_strength = old_strength * factor;
/// ```
#[inline]
pub fn hybrid_decay_factor(days_elapsed: f64, potentiated: bool) -> f32 {
    if days_elapsed <= 0.0 {
        return 1.0;
    }

    let beta = if potentiated {
        POWERLAW_BETA_POTENTIATED
    } else {
        POWERLAW_BETA
    };

    // Exponential rate for consolidation phase
    // Potentiated memories use slower exponential decay too
    let lambda = if potentiated {
        DECAY_LAMBDA_CONSOLIDATION * 0.5 // Half the rate for potentiated
    } else {
        DECAY_LAMBDA_CONSOLIDATION
    };

    if days_elapsed < DECAY_CROSSOVER_DAYS {
        // Consolidation phase: exponential decay
        // w(t) = w₀ × e^(-λt)
        (-lambda * days_elapsed).exp() as f32
    } else {
        // Long-term phase: power-law decay
        // First, calculate what value we'd have at crossover with exponential
        let value_at_crossover = (-lambda * DECAY_CROSSOVER_DAYS).exp();

        // Then apply power-law from crossover point
        // A(t) = A_cross × (t / t_cross)^(-β)
        let power_law_factor = (days_elapsed / DECAY_CROSSOVER_DAYS).powf(-beta);

        (value_at_crossover * power_law_factor) as f32
    }
}

/// Calculates the hybrid decay factor with custom parameters.
///
/// Use this for contexts that need different decay characteristics.
///
/// # Arguments
///
/// * `days_elapsed` - Time since last activation in days
/// * `crossover_days` - Days before switching from exponential to power-law
/// * `lambda` - Exponential decay rate for consolidation phase
/// * `beta` - Power-law exponent for long-term phase
///
/// # Example
///
/// ```ignore
/// // Faster decay for edge weights
/// let factor = hybrid_decay_factor_custom(days_elapsed, 1.0, 1.0, 0.6);
/// ```
#[inline]
pub fn hybrid_decay_factor_custom(
    days_elapsed: f64,
    crossover_days: f64,
    lambda: f64,
    beta: f64,
) -> f32 {
    if days_elapsed <= 0.0 {
        return 1.0;
    }

    if days_elapsed < crossover_days {
        // Consolidation phase: exponential decay
        (-lambda * days_elapsed).exp() as f32
    } else {
        // Long-term phase: power-law decay
        let value_at_crossover = (-lambda * crossover_days).exp();
        let power_law_factor = (days_elapsed / crossover_days).powf(-beta);
        (value_at_crossover * power_law_factor) as f32
    }
}

/// Calculates retention percentage for debugging/visualization.
///
/// Returns a human-readable percentage string showing retention at various time points.
#[allow(dead_code)]
pub fn retention_curve_debug(potentiated: bool) -> String {
    let days = [0.5, 1.0, 3.0, 7.0, 14.0, 30.0, 90.0, 365.0];
    let mode = if potentiated { "potentiated" } else { "normal" };

    let mut output = format!("Retention curve ({mode}):\n");
    for d in days {
        let factor = hybrid_decay_factor(d, potentiated);
        output.push_str(&format!("  Day {:>5.1}: {:>6.2}%\n", d, factor * 100.0));
    }
    output
}

/// Tier-aware decay factor for edge consolidation (3-tier memory model)
///
/// Each tier has different decay characteristics based on hippocampal-cortical research:
/// - L1 (Working): ~2.9%/hour decay (λ=0.029), max 48 hours
/// - L2 (Episodic): ~3.1%/day decay (λ=0.031), max 30 days
/// - L3 (Semantic): ~2%/month decay (λ=0.02/720h), near-permanent
///
/// # Arguments
///
/// * `hours_elapsed` - Time since last activation in hours
/// * `tier` - Memory tier (0=L1, 1=L2, 2=L3)
/// * `ltp_decay_factor` - LTP decay protection factor (1.0=none, 0.5=2x slower, 0.1=10x slower)
///
/// # Returns
///
/// Decay factor (0.0-1.0) and whether edge should be pruned
///
/// # PIPE-4 Update
///
/// Changed from `potentiated: bool` to `ltp_decay_factor: f32` to support
/// multi-scale LTP with graduated protection levels:
/// - LtpStatus::None → 1.0 (no protection)
/// - LtpStatus::Burst → 0.5 (2x slower decay, temporary)
/// - LtpStatus::Weekly → 0.3 (3x slower decay, moderate)
/// - LtpStatus::Full → 0.1 (10x slower decay, maximum)
#[inline]
pub fn tier_decay_factor(hours_elapsed: f64, tier: u8, ltp_decay_factor: f32) -> (f32, bool) {
    use crate::constants::*;

    if hours_elapsed <= 0.0 {
        return (1.0, false);
    }

    let (decay_rate, max_age_hours, prune_threshold) = match tier {
        0 => {
            // L1 Working: ~2.9%/hour decay (λ=0.029), max 48 hours
            (
                L1_DECAY_PER_HOUR as f64,
                (L1_MAX_AGE_HOURS as f64),
                L1_PRUNE_THRESHOLD,
            )
        }
        1 => {
            // L2 Episodic: ~3.1%/day decay (λ=0.031), max 30 days
            let decay_per_hour = L2_DECAY_PER_DAY as f64 / 24.0;
            (
                decay_per_hour,
                (L2_MAX_AGE_DAYS as f64) * 24.0,
                L2_PRUNE_THRESHOLD,
            )
        }
        _ => {
            // L3 Semantic (tier 2+): 2%/month decay, near-permanent
            let decay_per_hour = L3_DECAY_PER_MONTH as f64 / (30.0 * 24.0);
            // Max age: effectively unlimited (10 years)
            (decay_per_hour, 87600.0, L3_PRUNE_THRESHOLD)
        }
    };

    // PIPE-4: Apply graduated LTP protection
    // ltp_decay_factor of 0.5 = 2x slower, 0.1 = 10x slower, 1.0 = no protection
    let effective_rate = decay_rate * ltp_decay_factor as f64;

    // Exponential decay: w(t) = w₀ × e^(-λt)
    let decay_factor = (-effective_rate * hours_elapsed).exp() as f32;

    // Check if edge exceeded max age (should prune)
    // PIPE-4: Potentiated edges (ltp_decay_factor < 1.0) extend max age proportionally
    let effective_max_age = if ltp_decay_factor < 1.0 {
        max_age_hours / ltp_decay_factor as f64
    } else {
        max_age_hours
    };
    let should_prune = hours_elapsed > effective_max_age && decay_factor < prune_threshold;

    (decay_factor.max(0.001), should_prune)
}

// =============================================================================
// Phase 3 (cognitive-engine): per-type decay configuration with floors
// =============================================================================
//
// Upstream Shodh decay is a single global model — every edge and every node
// uses the same `(crossover, lambda, beta)`. Tier 1 needs different decay
// schedules per edge category and (post-Phase-4) per node ontology type, plus
// **decay floors** that pin structurally important memories above a minimum
// retention regardless of inactivity. Phase 3 adds the configuration surface
// and a floor-aware decay function. Existing call sites are unchanged — they
// keep using `hybrid_decay_factor`. Callers that want per-type behaviour opt
// in via `DecayConfig::params_for_edge_category()` + `decay_factor_with_params`.

/// Per-type decay parameters. Mirrors the three knobs of the hybrid model
/// — exponential consolidation phase length, exponential rate, power-law
/// exponent — and adds a `floor`: the minimum retention multiplier the
/// decay function will ever return for this type. A floor of `0.0` means
/// "no floor"; `0.5` means "this type retains at least 50% of its current
/// strength regardless of how long it has been inactive."
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DecayParams {
    /// Days before decay switches from exponential to power-law (≈ 3.0 in upstream).
    pub crossover_days: f64,
    /// Exponential rate during the consolidation phase (≈ 0.693 in upstream).
    pub lambda: f64,
    /// Power-law exponent during the long-term phase (≈ 0.5 in upstream).
    pub beta: f64,
    /// Minimum retention factor (0.0 = no floor, 1.0 = no decay at all).
    /// Acts as a clamp: `result = max(computed_decay, floor)`.
    pub floor: f32,
}

impl DecayParams {
    /// Default parameters that reproduce the upstream `hybrid_decay_factor`
    /// behaviour for non-potentiated edges. Used for any edge category
    /// without an explicit override in [`DecayConfig`].
    pub const fn upstream_default() -> Self {
        Self {
            crossover_days: DECAY_CROSSOVER_DAYS,
            lambda: DECAY_LAMBDA_CONSOLIDATION,
            beta: POWERLAW_BETA,
            floor: 0.0,
        }
    }

    /// Default parameters that reproduce the upstream potentiated curve.
    pub const fn upstream_potentiated() -> Self {
        Self {
            crossover_days: DECAY_CROSSOVER_DAYS,
            lambda: DECAY_LAMBDA_CONSOLIDATION * 0.5,
            beta: POWERLAW_BETA_POTENTIATED,
            floor: 0.0,
        }
    }
}

impl Default for DecayParams {
    fn default() -> Self {
        Self::upstream_default()
    }
}

/// Decay configuration: a default that mirrors upstream and a per-edge-category
/// override map. Loadable from TOML/JSON via [`DecayConfig::from_toml_str`].
///
/// Node-type overrides are deferred to Phase 4 (ontology tags) — once
/// `NodeType` exists, this struct will gain a `per_node_type` map. Until
/// then the `default` covers all nodes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecayConfig {
    /// Used when no override matches. Reproduces upstream behaviour by default.
    #[serde(default)]
    pub default: DecayParams,

    /// Per-edge-category overrides. Anything absent falls back to `default`.
    #[serde(default)]
    pub per_edge_category: HashMap<EdgeCategory, DecayParams>,
}

impl Default for DecayConfig {
    fn default() -> Self {
        Self {
            default: DecayParams::upstream_default(),
            per_edge_category: HashMap::new(),
        }
    }
}

impl DecayConfig {
    /// Look up the params for an edge category, falling back to `default`.
    pub fn params_for_edge_category(&self, category: EdgeCategory) -> DecayParams {
        self.per_edge_category
            .get(&category)
            .copied()
            .unwrap_or(self.default)
    }

    /// Parse a `DecayConfig` from a JSON string. Useful for `--decay-config`
    /// CLI flags and for inline test configs. JSON was chosen over TOML so
    /// the engine doesn't grow a new dependency just for config loading —
    /// `serde_json` is already in the build graph.
    pub fn from_json_str(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }

    /// Read a `DecayConfig` from a JSON file on disk.
    pub fn from_file(path: impl AsRef<std::path::Path>) -> std::io::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        Self::from_json_str(&contents)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

/// Hybrid decay with explicit per-type params and a retention floor.
///
/// Returns a value in `[params.floor, 1.0]`. The shape of the curve below
/// the floor is identical to [`hybrid_decay_factor_custom`]; the floor is
/// applied as a `max` clamp at the very end.
#[inline]
pub fn decay_factor_with_params(days_elapsed: f64, params: &DecayParams) -> f32 {
    let raw =
        hybrid_decay_factor_custom(days_elapsed, params.crossover_days, params.lambda, params.beta);
    raw.max(params.floor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_decay_at_zero() {
        assert_eq!(hybrid_decay_factor(0.0, false), 1.0);
        assert_eq!(hybrid_decay_factor(-1.0, false), 1.0);
    }

    #[test]
    fn test_exponential_phase() {
        // During consolidation (< 3 days), should be exponential
        let factor_1day = hybrid_decay_factor(1.0, false);
        let factor_2day = hybrid_decay_factor(2.0, false);

        // Exponential property: ratio should be constant
        let ratio_1_to_2 = factor_2day / factor_1day;
        let expected_ratio = (-DECAY_LAMBDA_CONSOLIDATION).exp() as f32;

        assert!((ratio_1_to_2 - expected_ratio).abs() < 0.01);
    }

    #[test]
    fn test_powerlaw_phase() {
        // After crossover (> 3 days), should be power-law
        let factor_7day = hybrid_decay_factor(7.0, false);
        let factor_14day = hybrid_decay_factor(14.0, false);

        // Power-law property: doubling time should give 2^(-β) ratio
        let ratio = factor_14day / factor_7day;
        let expected_ratio = 2.0_f64.powf(-POWERLAW_BETA) as f32;

        assert!((ratio - expected_ratio).abs() < 0.02);
    }

    #[test]
    fn test_continuity_at_crossover() {
        // Values just before and after crossover should be close
        let just_before = hybrid_decay_factor(DECAY_CROSSOVER_DAYS - 0.001, false);
        let just_after = hybrid_decay_factor(DECAY_CROSSOVER_DAYS + 0.001, false);

        assert!((just_before - just_after).abs() < 0.01);
    }

    #[test]
    fn test_potentiated_decays_slower() {
        let normal = hybrid_decay_factor(30.0, false);
        let potentiated = hybrid_decay_factor(30.0, true);

        // Potentiated should retain more
        assert!(potentiated > normal);
    }

    #[test]
    fn test_heavy_tail_retention() {
        // Key property: power-law has heavy tail
        // At 365 days, we should still have meaningful retention
        let year_retention = hybrid_decay_factor(365.0, false);
        let year_retention_potentiated = hybrid_decay_factor(365.0, true);

        // Normal: should be > 1%
        assert!(year_retention > 0.01);
        // Potentiated: should be > 5%
        assert!(year_retention_potentiated > 0.05);
    }

    #[test]
    fn test_custom_parameters() {
        // Test custom function with aggressive decay
        let aggressive = hybrid_decay_factor_custom(7.0, 1.0, 1.5, 0.7);
        let normal = hybrid_decay_factor(7.0, false);

        assert!(aggressive < normal);
    }

    #[test]
    fn test_tier_decay_factor_l1_with_and_without_ltp() {
        let (unprotected, _) = tier_decay_factor(24.0, 0, 1.0);
        let (protected, _) = tier_decay_factor(24.0, 0, 0.5);
        assert!(protected > unprotected);
    }

    #[test]
    fn test_tier_decay_factor_l1_prune_threshold() {
        let (factor_at_max_age, should_prune_at_max_age) = tier_decay_factor(48.0, 0, 1.0);
        // At max age boundary, L1 should still not prune because pruning requires "greater than" max age.
        assert!(factor_at_max_age > 0.1);
        assert!(!should_prune_at_max_age);

        let (factor_past_max_age, should_prune_past_max_age) = tier_decay_factor(96.0, 0, 1.0);
        assert!(factor_past_max_age < 0.1);
        assert!(should_prune_past_max_age);
    }

    #[test]
    fn test_tier_decay_factor_l3_long_tail() {
        let (factor_1y, prune_1y) = tier_decay_factor(365.0 * 24.0, 2, 1.0);
        assert!(factor_1y > 0.7);
        assert!(!prune_1y);

        let (factor_3y, prune_3y) = tier_decay_factor(3.0 * 365.0 * 24.0, 2, 1.0);
        assert!(factor_3y > 0.45);
        assert!(!prune_3y);
    }

    #[test]
    fn test_tier_decay_zero_and_negative_elapsed() {
        let (zero_factor, zero_prune) = tier_decay_factor(0.0, 1, 1.0);
        assert_eq!(zero_factor, 1.0);
        assert!(!zero_prune);

        let (neg_factor, neg_prune) = tier_decay_factor(-10.0, 1, 1.0);
        assert_eq!(neg_factor, 1.0);
        assert!(!neg_prune);
    }

    #[test]
    fn test_tier_decay_invalid_tier_defaults_to_l3() {
        let (invalid_tier, _) = tier_decay_factor(24.0, 9, 1.0);
        let (l3, _) = tier_decay_factor(24.0, 2, 1.0);
        assert_eq!(invalid_tier, l3);
    }

    // === Phase 3: per-type decay configuration with floors ===

    #[test]
    fn decay_params_default_matches_upstream_curve() {
        // The default DecayParams must reproduce the upstream
        // `hybrid_decay_factor(_, false)` curve exactly — otherwise
        // existing call sites that adopt the new API would silently change
        // behaviour.
        let params = DecayParams::default();
        for &days in &[0.0, 0.5, 1.0, 2.99, 3.0, 7.0, 30.0, 365.0] {
            let upstream = hybrid_decay_factor(days, false);
            let phase3 = decay_factor_with_params(days, &params);
            assert!(
                (upstream - phase3).abs() < 1e-6,
                "Default params should match upstream at day {days}: upstream={upstream}, phase3={phase3}"
            );
        }
    }

    #[test]
    fn decay_floor_clamps_long_inactivity() {
        // After a year of inactivity, the raw curve drops well below 10%.
        // With a 0.5 floor it must clamp at exactly 0.5.
        let raw = hybrid_decay_factor(365.0, false);
        assert!(
            raw < 0.5,
            "Sanity: raw 1-year retention should be below the 0.5 floor (got {raw})"
        );

        let with_floor = decay_factor_with_params(
            365.0,
            &DecayParams {
                floor: 0.5,
                ..DecayParams::default()
            },
        );
        assert!((with_floor - 0.5).abs() < 1e-6);
    }

    #[test]
    fn decay_floor_does_not_lift_fresh_memories() {
        // Floor only applies to decayed values — at t=0, retention is 1.0
        // and the floor must not pull it down or up artificially.
        let fresh = decay_factor_with_params(
            0.0,
            &DecayParams {
                floor: 0.5,
                ..DecayParams::default()
            },
        );
        assert_eq!(fresh, 1.0);
    }

    #[test]
    fn decay_config_lookup_falls_back_to_default() {
        let mut config = DecayConfig::default();
        config.per_edge_category.insert(
            EdgeCategory::Meta,
            DecayParams {
                floor: 0.7,
                ..DecayParams::default()
            },
        );

        // Configured category gets its override
        let meta_params = config.params_for_edge_category(EdgeCategory::Meta);
        assert_eq!(meta_params.floor, 0.7);

        // Unconfigured category falls back
        let causal_params = config.params_for_edge_category(EdgeCategory::Causal);
        assert_eq!(causal_params, DecayParams::default());
    }

    #[test]
    fn decay_config_per_category_diverges() {
        // Cognitive-engine motivating example: Meta::Contradicts edges should
        // decay slower (β=0.2, floor=0.4) so contradictions are not erased
        // by long inactivity.
        let mut config = DecayConfig::default();
        config.per_edge_category.insert(
            EdgeCategory::Meta,
            DecayParams {
                crossover_days: 3.0,
                lambda: 0.693,
                beta: 0.2,
                floor: 0.4,
            },
        );

        let causal = config.params_for_edge_category(EdgeCategory::Causal);
        let meta = config.params_for_edge_category(EdgeCategory::Meta);

        // After 90 days, Meta with its slower beta + floor should retain
        // much more than the default-configured Causal.
        let causal_at_90 = decay_factor_with_params(90.0, &causal);
        let meta_at_90 = decay_factor_with_params(90.0, &meta);
        assert!(
            meta_at_90 > causal_at_90 + 0.1,
            "Meta should retain materially more than Causal after 90d: meta={meta_at_90}, causal={causal_at_90}"
        );
    }

    #[test]
    fn decay_config_json_roundtrip() {
        let mut original = DecayConfig::default();
        original.per_edge_category.insert(
            EdgeCategory::Meta,
            DecayParams {
                crossover_days: 2.0,
                lambda: 0.5,
                beta: 0.3,
                floor: 0.4,
            },
        );

        let json = serde_json::to_string(&original).expect("encode");
        let decoded = DecayConfig::from_json_str(&json).expect("decode");
        assert_eq!(original, decoded);
    }

    #[test]
    fn decay_config_json_partial_override() {
        // Real-world config files only specify what's overridden — the
        // default block and absent categories must be filled in.
        let json = r#"{
            "per_edge_category": {
                "Meta": {
                    "crossover_days": 3.0,
                    "lambda": 0.693,
                    "beta": 0.2,
                    "floor": 0.4
                }
            }
        }"#;

        let config = DecayConfig::from_json_str(json).expect("decode");
        assert_eq!(config.default, DecayParams::upstream_default());
        assert_eq!(config.per_edge_category.len(), 1);
        assert_eq!(
            config
                .params_for_edge_category(EdgeCategory::Meta)
                .floor,
            0.4
        );
    }
}

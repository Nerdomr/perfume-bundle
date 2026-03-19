// ============================================================
// Perfume Bundle — Shopify Cart Transform Function (Rust)
// Logic: when exactly 4 "Bundle-Eligible" / Perfumes-collection
// items are in the cart, merge them into a single $100 bundle
// line and retain all 4 variants as children for inventory.
// ============================================================

use shopify_function::prelude::*;
use shopify_function::Result;

// Generated types from run.graphql via `shopify function typegen`
use crate::run::input::RunInput;
use crate::run::output::{
    CartTransform, CartLine, CartLineInput,
    CartLineCost, PriceAdjustment, PriceAdjustmentValue,
    ExpandedItem, MergeOperation,
};

mod run {
    pub mod input {
        include!(concat!(env!("OUT_DIR"), "/run_input.rs"));
    }
    pub mod output {
        include!(concat!(env!("OUT_DIR"), "/run_output.rs"));
    }
}

// --------------- constants ---------------
const BUNDLE_ELIGIBLE_TAG: &str = "Bundle-Eligible";
const REQUIRED_BUNDLE_SIZE: usize = 4;

/// Fixed bundle price in cents (USD). $100.00 = 10000
const BUNDLE_FIXED_PRICE_CENTS: i64 = 10_000;
/// Discount alternative: 15% (multiply unit price × 0.85)
/// Set USE_FIXED_PRICE = false below to switch modes.
const USE_FIXED_PRICE: bool = true;
const BUNDLE_DISCOUNT_PCT: f64 = 0.15; // 15 %

#[shopify_function]
fn run(input: RunInput) -> Result<CartTransform> {
    let lines = &input.cart.lines;

    // ── 1. Collect all bundle-eligible line items ──────────────────────────
    let eligible: Vec<&crate::run::input::CartLine> = lines
        .iter()
        .filter(|line| is_bundle_eligible(line))
        .collect();

    // ── 2. Count total quantity of eligible items ─────────────────────────
    let total_qty: i64 = eligible.iter().map(|l| l.quantity).sum();

    // Only trigger when we have EXACTLY 4 units
    if total_qty != REQUIRED_BUNDLE_SIZE as i64 {
        return Ok(CartTransform { operations: vec![] });
    }

    // ── 3. Build the bundle price ─────────────────────────────────────────
    let bundle_price = compute_bundle_price(&eligible);

    // ── 4. Build expanded children (one per eligible line) ───────────────
    //    Each child retains its original variant ID so Shopify can deduct
    //    inventory per scent correctly.
    let children: Vec<ExpandedItem> = eligible
        .iter()
        .map(|line| {
            let variant_id = match &line.merchandise {
                InputMerchandise::ProductVariant(v) => v.id.clone(),
                _ => String::new(),
            };
            ExpandedItem {
                merchandise_id: variant_id,
                quantity: line.quantity,
            }
        })
        .collect();

    // ── 5. Build the merge operation ──────────────────────────────────────
    let parent_variant_id = match &eligible[0].merchandise {
        InputMerchandise::ProductVariant(v) => v.id.clone(),
        _ => String::new(),
    };

    let merge_op = MergeOperation {
        parent_variant_id,
        // Cart line IDs being merged (so Shopify removes them)
        cart_lines: eligible
            .iter()
            .map(|l| CartLineInput { id: l.id.clone() })
            .collect(),
        title: Some(String::from("The Curated Perfume Set")),
        price: PriceAdjustment {
            value: if USE_FIXED_PRICE {
                PriceAdjustmentValue::FixedPricePerUnit(
                    shopify_function::types::Decimal(
                        format!("{:.2}", bundle_price / 100.0)
                    )
                )
            } else {
                PriceAdjustmentValue::Percentage(
                    shopify_function::types::Decimal(
                        format!("{:.2}", BUNDLE_DISCOUNT_PCT * 100.0)
                    )
                )
            },
        },
        image: None,
        // Children preserve per-variant inventory tracking
        expanded_cart_items: children,
    };

    Ok(CartTransform {
        operations: vec![
            shopify_function::types::CartOperation::Merge(merge_op)
        ],
    })
}

// ─────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────

/// Returns true when a line belongs to the Perfumes collection
/// OR carries the "Bundle-Eligible" product tag.
fn is_bundle_eligible(line: &crate::run::input::CartLine) -> bool {
    match &line.merchandise {
        InputMerchandise::ProductVariant(variant) => {
            let in_collection = variant.product.in_any_collection;
            let has_tag = variant
                .product
                .tags
                .iter()
                .any(|t| t.as_str() == BUNDLE_ELIGIBLE_TAG);
            in_collection || has_tag
        }
        _ => false,
    }
}

/// Sum individual retail prices across all eligible lines.
/// Used when USE_FIXED_PRICE = false to compute the 15% discount base.
fn compute_bundle_price(lines: &[&crate::run::input::CartLine]) -> f64 {
    if USE_FIXED_PRICE {
        return BUNDLE_FIXED_PRICE_CENTS as f64; // already in cents
    }

    let total_cents: f64 = lines
        .iter()
        .map(|line| {
            let unit_price = match &line.merchandise {
                InputMerchandise::ProductVariant(v) => {
                    v.price.amount.parse::<f64>().unwrap_or(0.0) * 100.0
                }
                _ => 0.0,
            };
            unit_price * line.quantity as f64
        })
        .sum();

    // Apply 15 % discount
    (total_cents * (1.0 - BUNDLE_DISCOUNT_PCT)).round()
}

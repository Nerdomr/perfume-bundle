# Perfume Bundle — Shopify Cart Transform Function
## Deployment Guide (Shopify CLI 3.x)

---

## Prerequisites

| Tool | Version | Install |
|------|---------|---------|
| Node.js | ≥ 18 | https://nodejs.org |
| Rust | stable | https://rustup.rs |
| wasm32 target | — | `rustup target add wasm32-wasip1` |
| Shopify CLI | 3.x | `npm install -g @shopify/cli` |

---

## Step 1 — Create a Custom App in the Partner Dashboard

1. Go to https://partners.shopify.com → **Apps → Create app → Custom app**.
2. Set your app name to "Perfume Bundle".
3. Copy the **Client ID** and paste it into `shopify.app.toml → client_id`.
4. Under **Configuration → API access**, grant:
   - `write_cart_transforms`
   - `read_products`
5. Click **Save**.

---

## Step 2 — Link the CLI to your app

```bash
# From the project root
shopify app dev --reset
# Follow the prompts: select your Partner org → select your app → select your dev store
```

This authenticates your CLI session and links the local project to the remote app.

---

## Step 3 — Replace the placeholder Collection ID

Open `extensions/perfume-bundle/src/run.graphql` and replace:

```
PERFUMES_COLLECTION_ID
```

with your actual Shopify collection GID. You can find it in:

- **Shopify Admin → Products → Collections → [Your collection]**
- The URL contains the ID: `.../collections/123456789` → GID = `gid://shopify/Collection/123456789`

---

## Step 4 — Build the Rust WASM

```bash
cd extensions/perfume-bundle
cargo build --release --target wasm32-wasip1
```

The compiled function lives at:
`target/wasm32-wasip1/release/perfume_bundle.wasm`

To run local tests against the function:

```bash
# From the extension directory
shopify function run
# Paste sample input JSON when prompted
```

Sample input to test the merge trigger:

```json
{
  "cart": {
    "lines": [
      { "id": "line_1", "quantity": 1, "merchandise": { "id": "gid://shopify/ProductVariant/11111", "price": { "amount": "35.00", "currencyCode": "USD" }, "product": { "id": "gid://shopify/Product/1", "title": "Oud Sublime", "tags": ["Bundle-Eligible"], "inAnyCollection": true } } },
      { "id": "line_2", "quantity": 1, "merchandise": { "id": "gid://shopify/ProductVariant/22222", "price": { "amount": "35.00", "currencyCode": "USD" }, "product": { "id": "gid://shopify/Product/2", "title": "Blanche Iris", "tags": ["Bundle-Eligible"], "inAnyCollection": true } } },
      { "id": "line_3", "quantity": 1, "merchandise": { "id": "gid://shopify/ProductVariant/33333", "price": { "amount": "36.00", "currencyCode": "USD" }, "product": { "id": "gid://shopify/Product/3", "title": "Rose Noire", "tags": ["Bundle-Eligible"], "inAnyCollection": true } } },
      { "id": "line_4", "quantity": 1, "merchandise": { "id": "gid://shopify/ProductVariant/44444", "price": { "amount": "37.00", "currencyCode": "USD" }, "product": { "id": "gid://shopify/Product/4", "title": "Santal Dusk", "tags": ["Bundle-Eligible"], "inAnyCollection": true } } }
    ]
  }
}
```

Expected output: a single `merge` operation targeting the 4 line IDs with `$100.00` price.

---

## Step 5 — Deploy to Shopify

```bash
# From the project root
shopify app deploy
```

This command:
1. Compiles your Rust function to WASM.
2. Uploads the WASM binary to Shopify's infrastructure.
3. Registers the Cart Transform API function under your app.

The CLI will print a function ID on success, e.g.:
```
✓ Function perfume-bundle deployed (ID: 01JABCD1234XYZ)
```

---

## Step 6 — Activate the Cart Transform

Cart Transform functions must be activated via the Admin API. Run this GraphQL mutation once in your app's OAuth callback or setup flow:

```graphql
mutation {
  cartTransformCreate(functionId: "01JABCD1234XYZ") {
    cartTransform {
      id
      functionId
    }
    userErrors {
      field
      message
    }
  }
}
```

You can run this in **Shopify Admin → Apps → [Your app] → GraphiQL explorer**.

---

## Step 7 — Add the Liquid Section to your theme

1. Copy `sections/perfume-bundle-tracker.liquid` → your theme's `sections/` folder.
2. Copy `assets/perfume-bundle-tracker.css` → your theme's `assets/` folder.
3. In the theme editor (**Online Store → Themes → Customize**):
   - Click **Add section**.
   - Find **"Perfume Bundle Tracker"**.
   - Set the **Collection** to your Perfumes collection.
4. Save.

---

## Architecture overview

```
Storefront (Liquid section)
  └─ User selects 4 perfumes
  └─ /cart/add.js called with 4 variant IDs
        ↓
Shopify Cart
  └─ Cart Transform Function fires (Rust WASM)
  └─ Checks: are exactly 4 Bundle-Eligible lines present?
  └─ YES → MergeOperation emitted:
      ├─ parent: variant[0]
      ├─ price: $100.00 fixed
      └─ children: all 4 variants (inventory deducted per scent)
        ↓
Checkout
  └─ Shows single "Curated Perfume Set" line @ $100
  └─ Each child variant's inventory is reserved independently
```

---

## Switching between fixed price and discount

In `src/run.rs`, find the constant:

```rust
const USE_FIXED_PRICE: bool = true;
```

- `true`  → Bundle is always **$100.00** regardless of individual prices.
- `false` → Bundle is the sum of retail prices **minus 15%** (configurable via `BUNDLE_DISCOUNT_PCT`).

After changing, rebuild: `cargo build --release --target wasm32-wasip1` then `shopify app deploy`.

---

## Tagging products

For a product to be eligible, either:

- It must be **in your Perfumes collection** (GID set in `run.graphql`), **OR**
- It must carry the product tag **`Bundle-Eligible`** (exact match, case-sensitive).

Both conditions are OR'd — satisfying either makes the line eligible.

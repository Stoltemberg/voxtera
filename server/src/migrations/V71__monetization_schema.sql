-- Phase 0/1: Monetization schema
-- Adds premium currency (Cristais) persistence and audit logging.
--
-- Currency is stored per-account (player UUID), not per-character,
-- because cosmetics are shared across all characters on an account.
--
-- NOTE: For Phase 1 we also add a `cristais` column to the `character`
-- table as a transitional measure so the component can be persisted
-- alongside other character data. In Phase 4 (Stripe) this will be
-- migrated to the per-account `account_currency` table exclusively.

-- Add cristais column to character table (Phase 1 transitional)
ALTER TABLE character ADD COLUMN IF NOT EXISTS cristais INTEGER NOT NULL DEFAULT 0;

-- Per-account premium currency balance
CREATE TABLE IF NOT EXISTS account_currency (
    -- Player UUID (same as character.player_uuid)
    player_uuid TEXT NOT NULL PRIMARY KEY,
    -- Current Cristais balance
    cristais INTEGER NOT NULL DEFAULT 0,
    -- When the balance was last modified (Unix timestamp, seconds)
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Immutable audit log of every currency change.
-- Every grant, spend, admin adjustment, and Stripe purchase is
-- recorded here. This table is append-only — no UPDATE or DELETE.
CREATE TABLE IF NOT EXISTS currency_change_log (
    -- Auto-increment ID
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    -- Player UUID whose wallet changed
    player_uuid TEXT NOT NULL,
    -- Previous balance before the change
    old_balance INTEGER NOT NULL,
    -- New balance after the change
    new_balance INTEGER NOT NULL,
    -- Amount of the change (positive for grant, negative for spend)
    delta INTEGER NOT NULL,
    -- Why the change happened (serialized CurrencyChangeReason as JSON)
    reason TEXT NOT NULL,
    -- Unix timestamp (seconds)
    timestamp INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    -- Index for querying a player's history
    FOREIGN KEY (player_uuid) REFERENCES account_currency(player_uuid)
);

CREATE INDEX IF NOT EXISTS idx_currency_log_player
    ON currency_change_log(player_uuid, timestamp DESC);

-- Stripe purchase tracking (Phase 4 preparation — empty for now)
-- Used to ensure idempotent webhook processing: if a Stripe event
-- has already been processed, we skip it.
CREATE TABLE IF NOT EXISTS stripe_events_processed (
    -- Stripe event ID (e.g. "evt_12345")
    stripe_event_id TEXT NOT NULL PRIMARY KEY,
    -- Stripe checkout session ID (e.g. "cs_test_12345")
    stripe_session_id TEXT,
    -- Type of event (e.g. "checkout.session.completed")
    event_type TEXT NOT NULL,
    -- Player UUID that received the currency
    player_uuid TEXT,
    -- Amount of cristais granted
    cristais_granted INTEGER,
    -- When the webhook was processed
    processed_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Cosmetic ownership tracking (Phase 2 preparation — empty for now)
-- Records which cosmetics a player account owns. This is the source
-- of truth for the wardrobe: if a cosmetic_id is in this table for
-- a given player_uuid, the player owns it.
CREATE TABLE IF NOT EXISTS account_cosmetics (
    -- Auto-increment ID
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    -- Player UUID
    player_uuid TEXT NOT NULL,
    -- Cosmetic identifier (matches CosmeticId in Rust)
    cosmetic_id TEXT NOT NULL,
    -- How the cosmetic was acquired (shop_purchase, bp_reward, admin_gift, founder_pack)
    acquisition_method TEXT NOT NULL DEFAULT 'admin_gift',
    -- When the cosmetic was granted
    granted_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    -- Unique constraint: a player can only own a cosmetic once
    UNIQUE (player_uuid, cosmetic_id)
);

CREATE INDEX IF NOT EXISTS idx_account_cosmetics_player
    ON account_cosmetics(player_uuid);

-- Purchase history (both cristais and real money)
CREATE TABLE IF NOT EXISTS purchase_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    -- Player UUID
    player_uuid TEXT NOT NULL,
    -- What was purchased (cosmetic_id, bp_premium, cristais_package)
    item_purchased TEXT NOT NULL,
    -- Currency used: "cristais" or "brl"
    currency TEXT NOT NULL,
    -- Amount paid (in cristais or BRL cents)
    amount_paid INTEGER NOT NULL,
    -- When the purchase happened
    purchased_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_purchase_history_player
    ON purchase_history(player_uuid, purchased_at DESC);

//! Premium currency system — "Cristais" (Crystals).
//!
//! This is the only premium currency in Voxtera. It is used to buy
//! cosmetics and battle pass tracks. It is **never** used for
//! gameplay-affecting purchases (stats, XP, drop rate, etc.).
//!
//! Phase 0/1: Only the types, component, and spend/grant logic.
//! Phase 4: Stripe integration to buy Cristais with real money.

use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage, VecStorage};

/// Premium currency component attached to each player entity.
///
/// `Cristais` are per-account (player UUID), not per-character, but for
/// Phase 1 simplicity we store them as a component on the player entity
/// and persist them in the character table. In Phase 4, when Stripe is
/// integrated, we will migrate to a per-account table keyed by UUID.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PremiumCurrency {
    /// Current balance of Cristais.
    cristais: u32,
}

impl Default for PremiumCurrency {
    fn default() -> Self { Self { cristais: 0 } }
}

impl Component for PremiumCurrency {
    type Storage = DerefFlaggedStorage<Self, VecStorage<Self>>;
}

impl PremiumCurrency {
    /// Create a new wallet with the given balance.
    pub fn new(cristais: u32) -> Self { Self { cristais } }

    /// Current balance.
    pub fn cristais(&self) -> u32 { self.cristais }

    /// Add cristais to the wallet. Used by admin commands and Stripe
    /// webhook (Phase 4).
    ///
    /// Returns the new balance. Saturates on overflow to prevent
    /// any exploits.
    pub fn grant(&mut self, amount: u32) -> u32 {
        self.cristais = self.cristais.saturating_add(amount);
        self.cristais
    }

    /// Attempt to spend cristais. Returns `Err` if the wallet has
    /// insufficient funds — the balance is NOT modified in that case.
    ///
    /// On success, returns the new balance.
    pub fn spend(&mut self, cost: u32) -> Result<u32, CurrencyError> {
        if self.cristais < cost {
            return Err(CurrencyError::InsufficientFunds {
                have: self.cristais,
                need: cost,
            });
        }
        self.cristais -= cost;
        Ok(self.cristais)
    }

    /// Set balance directly. Used only by persistence layer on load.
    pub fn set_balance(&mut self, amount: u32) { self.cristais = amount; }
}

/// Error returned when a currency operation fails.
#[derive(Debug, Clone, PartialEq)]
pub enum CurrencyError {
    /// Not enough cristais to complete the purchase.
    InsufficientFunds {
        /// What the player currently has
        have: u32,
        /// What the purchase costs
        need: u32,
    },
}

impl std::fmt::Display for CurrencyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InsufficientFunds { have, need } => write!(
                f,
                "Cristais insuficientes: tem {}, precisa de {}",
                have, need
            ),
        }
    }
}

impl std::error::Error for CurrencyError {}

/// Reason for a currency change, used for audit logging.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CurrencyChangeReason {
    /// Admin command `/givecrystals`
    AdminGrant { admin_uuid: String },
    /// Admin command `/removecrystals`
    AdminRemove { admin_uuid: String },
    /// Purchase in the cosmetic shop (Phase 3)
    ShopPurchase { cosmetic_id: String, cost: u32 },
    /// Battle Pass premium track purchase (Phase 5)
    BattlePassPurchase { season_id: u32 },
    /// Stripe webhook — real money purchase (Phase 4)
    StripePurchase { session_id: String, package_id: String },
    /// Battle Pass reward claim (Phase 5)
    BPReward { season_id: u32, tier: u8 },
    /// Manual adjustment by system (debug/compensation)
    SystemAdjustment { note: String },
}

/// An entry in the currency change audit log.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CurrencyChangeLog {
    /// UUID of the player whose wallet changed
    pub player_uuid: String,
    /// Previous balance
    pub old_balance: u32,
    /// New balance
    pub new_balance: u32,
    /// Amount of the change (positive for grant, negative for spend)
    pub delta: i64,
    /// Why the change happened
    pub reason: CurrencyChangeReason,
    /// Unix timestamp (seconds)
    pub timestamp: i64,
}

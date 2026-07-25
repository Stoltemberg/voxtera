//! Cosmetic system types — shared between server and client.
//!
//! Cosmetics are purely visual items that never affect gameplay.
//! They are owned per-account (player UUID) and can be equipped
//! in specific slots. This module defines only the types; the
//! actual inventory storage and UI are implemented in later phases.

use serde::{Deserialize, Serialize};

/// Unique identifier for a cosmetic item.
/// In production these will come from RON asset definitions.
/// For now we use a simple string-based ID.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct CosmeticId(pub String);

impl CosmeticId {
    pub fn new(id: impl Into<String>) -> Self { Self(id.into()) }
}

impl std::fmt::Display for CosmeticId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Categories of cosmetic items. Each maps to an equipment slot
/// in the wardrobe UI (Phase 2).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize, enum_map::Enum)]
pub enum CosmeticCategory {
    /// Helmets, hats, crowns — replaces head visual
    Head,
    /// Chest armor skins, robes, costumes — replaces body visual
    Body,
    /// Gloves, gauntlets — replaces hands visual
    Hands,
    /// Boots, greaves — replaces feet visual
    Feet,
    /// Cape, wings, backpack — back slot
    Back,
    /// Non-combat pets that follow the player
    Pet,
    /// Visual mounts (no speed bonus)
    Mount,
    /// Social emotes (dance, wave, sit, etc.)
    Emote,
    /// Title displayed above character name
    Title,
    /// Movement trail effects (sparkles, fire, etc.)
    Trail,
    /// Furniture and decorations for player housing
    HomeDecor,
    /// Death animation/effect
    DeathEffect,
}

/// Rarity tier for cosmetics. Purely visual (border colour, glow).
/// Does NOT affect gameplay or stats.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum CosmeticRarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
    Mythic,
}

impl CosmeticRarity {
    /// Returns the display colour for this rarity (R, G, B, A) in linear space.
    pub fn color(self) -> [f32; 4] {
        match self {
            Self::Common => [0.7, 0.7, 0.7, 1.0],
            Self::Uncommon => [0.3, 0.8, 0.3, 1.0],
            Self::Rare => [0.3, 0.5, 0.9, 1.0],
            Self::Epic => [0.7, 0.3, 0.9, 1.0],
            Self::Legendary => [0.9, 0.7, 0.2, 1.0],
            Self::Mythic => [0.9, 0.3, 0.3, 1.0],
        }
    }
}

/// A cosmetic item definition (loaded from assets in Phase 2).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CosmeticDef {
    pub id: CosmeticId,
    pub category: CosmeticCategory,
    pub rarity: CosmeticRarity,
    /// Display name i18n key, e.g. "cosmetic-flame-helm"
    pub name_key: String,
    /// Icon image asset path
    pub icon: String,
    /// 3D model asset path (for character preview)
    pub model: String,
    /// Which season/event this cosmetic belongs to (for filtering)
    pub season: Option<String>,
    /// Price in Cristais (premium currency). None = not for sale.
    pub price: Option<u32>,
}

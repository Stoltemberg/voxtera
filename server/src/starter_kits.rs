//! Starter Kit for new Voxtera players
//!
//! Gives basic equipment to players on their first login.

use common::comp::{
    Inventory,
    item::Item,
};
use tracing::info;

/// Give starter items to a new player's inventory.
/// Only called once per character (first login).
pub fn give_starter_items(inventory: &mut Inventory) {
    // Helper to give an item by asset path if it exists
    let give = |inventory: &mut Inventory, path: &str, amount: u16| {
        match Item::new_from_asset(path) {
            Ok(item) => {
                for _ in 0..amount {
                    inventory.push(item.clone());
                }
                info!(path, amount, "Gave starter item");
            },
            Err(_) => {
                info!(path, "Starter item not found, skipping");
            },
        }
    };

    // Weapon: simple sword
    give(inventory, "common.items.weapons.sword.starter_sword", 1);

    // Armor: basic leather set
    give(inventory, "common.items.armor.misc.chest.leather_chest", 1);
    give(inventory, "common.items.armor.misc.legs.leather_legs", 1);
    give(inventory, "common.items.armor.misc.feet.leather_boots", 1);

    // Food
    give(inventory, "common.items.food.apple.apple", 5);
    give(inventory, "common.items.food.bread.bread", 3);

    // Utility: lantern
    give(inventory, "common.items.tools.lantern.simple_lantern", 1);

    info!("Starter kit delivered");
}

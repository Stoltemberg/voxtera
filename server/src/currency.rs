//! Server-side premium currency (Cristais) operations.
//!
//! Provides atomic spend/grant helpers that operate on the ECS
//! `PremiumCurrency` component and notify the client via
//! `ServerGeneral::CurrencyChange`. All operations are atomic at the
//! Rust level — they either succeed and return the new balance, or
//! fail and leave the wallet untouched.

use common::{
    comp::{CurrencyChangeReason, CurrencyError, PremiumCurrency},
    uid::Uid,
};
use common_net::{msg::ServerGeneral, sync::WorldSyncExt};
use specs::{Entity as EcsEntity, WorldExt};

use crate::Server;

/// Result of a spend attempt.
#[derive(Debug, Clone)]
pub enum SpendResult {
    /// Spend succeeded, contains the new balance.
    Ok { new_balance: u32 },
    /// Player has insufficient funds.
    InsufficientFunds {
        /// Player's current balance.
        have: u32,
        /// Amount that was requested.
        need: u32,
    },
    /// Target player entity has no `PremiumCurrency` component.
    /// Should not happen if the server registered the component on login.
    NoWallet,
}

/// Attempt to spend `cost` cristais from the wallet of the entity
/// identified by `target`. On success, the new balance is sent to the
/// client via `ServerGeneral::CurrencyChange`.
///
/// The balance is never mutated unless the spend succeeds.
pub fn try_spend(
    server: &mut Server,
    target: EcsEntity,
    cost: u32,
    reason: CurrencyChangeReason,
) -> SpendResult {
    let mut storage = server
        .state
        .ecs_mut()
        .write_storage::<PremiumCurrency>();

    let Some(mut wallet) = storage.get_mut(target) else {
        return SpendResult::NoWallet;
    };

    let result = wallet.spend(cost);
    // Drop the storage borrow before sending notifications.
    drop(storage);

    match result {
        Ok(new_balance) => {
            // Notify the target's client so the HUD updates.
            server.notify_client(
                target,
                ServerGeneral::CurrencyChange(new_balance, reason),
            );
            SpendResult::Ok { new_balance }
        },
        Err(CurrencyError::InsufficientFunds { have, need }) => SpendResult::InsufficientFunds {
            have,
            need,
        },
    }
}

/// Like `try_spend` but resolves the target by `Uid` first. If the
/// uid cannot be resolved to a live entity, returns
/// `SpendResult::NoWallet`.
pub fn try_spend_by_uid(
    server: &mut Server,
    target_uid: Uid,
    cost: u32,
    reason: CurrencyChangeReason,
) -> SpendResult {
    let target = {
        let uid_alloc = server.state.ecs().read_resource::<common::uid::IdMaps>();
        uid_alloc.uid_entity(target_uid)
    };
    match target {
        Some(e) => try_spend(server, e, cost, reason),
        None => SpendResult::NoWallet,
    }
}

/// Grant `amount` cristais to a player by entity. Saturates on
/// overflow to avoid exploits. Notifies the client so the HUD updates.
pub fn try_grant(
    server: &mut Server,
    target: EcsEntity,
    amount: u32,
    reason: CurrencyChangeReason,
) -> Option<u32> {
    let mut storage = server
        .state
        .ecs_mut()
        .write_storage::<PremiumCurrency>();

    let new_balance = storage.get_mut(target)?.grant(amount);
    drop(storage);

    server.notify_client(
        target,
        ServerGeneral::CurrencyChange(new_balance, reason),
    );
    Some(new_balance)
}

/// Convenience: get the current balance of an entity's wallet.
pub fn balance(server: &Server, target: EcsEntity) -> Option<u32> {
    server
        .state
        .ecs()
        .read_storage::<PremiumCurrency>()
        .get(target)
        .map(|w| w.cristais())
}
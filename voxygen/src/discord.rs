use std::time::{Duration, SystemTime};

use common::terrain::SiteKindMeta;
use discord_sdk::{
    self as ds, activity,
    activity::{ActivityArgs, ActivityBuilder},
};
use i18n::{LocalizationGuard, LocalizationHandle};
use tokio::{
    sync::mpsc::{UnboundedSender, unbounded_channel},
    time::{MissedTickBehavior, interval},
};
use tracing::{debug, info, warn};

/// Discord app id
///
/// **Note:** currently a private app created for testing purposes, can be
/// shared to a team or replaced entirely later on
const DISCORD_APP_ID: ds::AppId = 1006661232465563698;

/// Discord presence update command
#[derive(Debug, Clone)]
pub enum ActivityUpdate {
    /// Clear the current Discord activity and exit the activity task
    Clear,
    /// Set the activity to "In Main Menu"
    MainMenu,
    /// Set the activity to "In Character Selection"
    CharacterSelection,
    /// Set the activity to "Playing Singleplayer"
    JoinSingleplayer,
    /// Set the activity to "Playing Multiplayer"
    JoinServer(String),
    /// Set the large asset text to the location name
    NewLocation {
        chunk_name: String,
        site: SiteKindMeta,
    },
}

impl ActivityUpdate {
    /// Rich Presence asset keys: the backgrounds used in the main menu and
    /// loading screen
    ///
    /// TODO: randomize images? use them according to the current biome?
    const ASSETS: [&'static str; 15] = [
        "bg_main", "bg_1", "bg_2", "bg_3", "bg_4", "bg_5", "bg_6", "bg_7", "bg_8", "bg_9",
        "bg_10", "bg_11", "bg_12", "bg_13", "bg_14",
    ];
    /// Rich Presence character screen asset key
    const CHARACTER_SCREEN_ASSET: &'static str = "character_screen";
    /// Rich Presence logo asset key
    const LOGO_ASSET: &'static str = "logo";

    /// Edit the current activity args according to the command in `self`.
    ///
    /// - For `MainMenu`, `CharacterSelection`, `JoinSingleplayer` and
    ///   `JoinServer(name)`: create a new activity and discard the previous one
    /// - For `NewLocation` and `LevelUp`: update the current activity
    fn edit_activity(self, args: &mut ActivityArgs, i18n: &LocalizationGuard) {
        use ActivityUpdate::*;

        match self {
            Clear => (),
            MainMenu => {
                *args = ActivityBuilder::default()
                    .start_timestamp(SystemTime::now())
                    .state(i18n.get_msg("discord-mainmenu-state").to_string())
                    .details(i18n.get_msg("discord-mainmenu-details").to_string())
                    .assets(
                        activity::Assets::default().large(Self::LOGO_ASSET, Option::<&str>::None),
                    )
                    .into();
            },
            CharacterSelection => {
                *args = ActivityBuilder::default()
                    .start_timestamp(SystemTime::now())
                    .state(i18n.get_msg("discord-charselect-state").to_string())
                    .details(i18n.get_msg("discord-charselect-details").to_string())
                    .assets(
                        activity::Assets::default()
                            .large(Self::CHARACTER_SCREEN_ASSET, Option::<&str>::None)
                            .small(Self::LOGO_ASSET, Option::<&str>::None),
                    )
                    .into();
            },
            JoinSingleplayer => {
                *args = ActivityBuilder::default()
                    .start_timestamp(SystemTime::now())
                    .details(i18n.get_msg("discord-singleplayer-details").to_string())
                    .assets(
                        activity::Assets::default()
                            .large(Self::ASSETS[9], Option::<&str>::None)
                            .small(Self::LOGO_ASSET, Option::<&str>::None),
                    )
                    .into();
            },
            JoinServer(server_name) => {
                let state = i18n.get_msg_ctx(
                    "discord-multiplayer-state",
                    &i18n::fluent_args! {
                        "server" => &*server_name,
                    },
                );
                *args = ActivityBuilder::default()
                    .start_timestamp(SystemTime::now())
                    .state(state.to_string())
                    .details(i18n.get_msg("discord-multiplayer-details").to_string())
                    .assets(
                        activity::Assets::default()
                            .large(Self::ASSETS[1], Option::<&str>::None)
                            .small(Self::LOGO_ASSET, Option::<&str>::None),
                    )
                    .into();
            },
            NewLocation { chunk_name, site } => {
                use common::terrain::site::{
                    DungeonKindMeta::*, SettlementKindMeta::*, SiteKindMeta::*,
                };

                let location = match site {
                    Dungeon(Gnarling) => i18n.get_msg_ctx(
                        "discord-location-hunting-gnarlings",
                        &i18n::fluent_args! { "location" => &*chunk_name },
                    ),
                    Dungeon(Adlet) => i18n.get_msg_ctx(
                        "discord-location-finding-yeti",
                        &i18n::fluent_args! { "location" => &*chunk_name },
                    ),
                    Dungeon(SeaChapel) => i18n.get_msg_ctx(
                        "discord-location-gathering-sea-treasures",
                        &i18n::fluent_args! { "location" => &*chunk_name },
                    ),
                    Dungeon(Terracotta) => i18n.get_msg_ctx(
                        "discord-location-exploring-ruins",
                        &i18n::fluent_args! { "location" => &*chunk_name },
                    ),
                    Cave => i18n.get_msg("discord-location-in-cave"),
                    Settlement(Default) => i18n.get_msg_ctx(
                        "discord-location-visiting",
                        &i18n::fluent_args! { "location" => &*chunk_name },
                    ),
                    Settlement(CliffTown) => i18n.get_msg_ctx(
                        "discord-location-climbing-towers",
                        &i18n::fluent_args! { "location" => &*chunk_name },
                    ),
                    Settlement(DesertCity) => i18n.get_msg_ctx(
                        "discord-location-hiding-from-sun",
                        &i18n::fluent_args! { "location" => &*chunk_name },
                    ),
                    Settlement(SavannahTown) => i18n.get_msg_ctx(
                        "discord-location-shopping-at-market",
                        &i18n::fluent_args! { "location" => &*chunk_name },
                    ),
                    Settlement(CoastalTown) => i18n.get_msg_ctx(
                        "discord-location-dipping-feet",
                        &i18n::fluent_args! { "location" => &*chunk_name },
                    ),
                    _ => i18n.get_msg_ctx(
                        "discord-location-in",
                        &i18n::fluent_args! { "location" => &*chunk_name },
                    ),
                };

                args.activity.as_mut().map(|a| {
                    a.assets.as_mut().map(|assets| {
                        assets.large_text = Some(location.to_string());
                    })
                });
            },
        }
    }
}

/// A channel to the background task that updates the Discord activity.
pub enum Discord {
    /// Active state, receiving updates
    Active {
        /// The channel to communicate with the tokio task
        channel: UnboundedSender<ActivityUpdate>,
        /// Current chunk name, cached to check for updates
        current_chunk_name: Option<String>,
        /// Current site, cached to check for updates
        current_site: SiteKindMeta,
    },
    /// Inactive state: either the Discord app could not be contacted, is not
    /// installed, or was disconnected
    Inactive,
}

impl Discord {
    /// Start a background [tokio task](tokio::task) that will update the
    /// Discord activity every 4 seconds (due to rate limits) if it has
    /// changed.
    ///
    /// The [`update`](Discord::update) method can be used on the returned
    /// struct to update the Discord activity via a channel command
    pub fn start(rt: &tokio::runtime::Runtime, i18n: LocalizationHandle) -> Self {
        let (sender, mut receiver) = unbounded_channel::<ActivityUpdate>();

        rt.spawn(async move {
            let (wheel, handler) = ds::wheel::Wheel::new(Box::new(|err| {
                warn!(error = ?err, "Encountered an error while connecting to Discord");
            }));

            let mut user = wheel.user();

            let discord = match ds::Discord::new(
                ds::DiscordApp::PlainId(DISCORD_APP_ID),
                ds::Subscriptions::ACTIVITY,
                Box::new(handler),
            ) {
                Ok(ds) => {
                    if let Err(err) = user.0.changed().await {
                        warn!(err = ?err, "Could not execute handshake to Discord");
                        // If no handshake is received, exit the task immediately
                        return;
                    }
                    info!("Connected to Discord");
                    ds
                },
                Err(err) => {
                    info!(err = ?err, "Could not connect to Discord app");
                    // If no Discord app was found, exit the task immediately
                    return;
                },
            };

            let mut args = ActivityArgs::default();
            let mut has_changed = false;
            let mut interval = interval(Duration::from_secs(4));
            interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

            loop {
                // Check every four seconds if the activity needs to change
                tokio::select! {
                    biased; // to save the CPU cost of selecting a random branch

                    _ = interval.tick(), if has_changed => {
                        has_changed = false;
                        let activity = args.activity.clone();
                        match discord.update_activity(args).await {
                            Err(err) => {
                                warn!(error = ?err, "Could not update Discord activity");
                            }
                            Ok(Some(new_activity)) => {
                                debug!(new_activity = ?new_activity, "Updated Discord activity");
                            },
                            Ok(None) => ()
                        }
                        args = ActivityArgs::default();
                        args.activity = activity;
                    }
                    update = receiver.recv() => match update {
                        None | Some(ActivityUpdate::Clear) => {
                            match discord.clear_activity().await {
                                Ok(_) => {
                                    info!("Cleared Discord activity");
                                },
                                Err(err) => {
                                    warn!(error = ?err, "Failed to clear Discord activity")
                                }
                            }
                            return;
                        },
                        Some(update) => {
                            // Get the current localization snapshot
                            let i18n = i18n.read();
                            update.edit_activity(&mut args, &i18n);
                            has_changed = true;
                        },
                    }
                }
            });
        });

        Self::Active {
            channel: sender,
            current_chunk_name: None,
            current_site: SiteKindMeta::Void,
        }
    }

    /// Send an activity update to the background task
    #[inline]
    fn update(&mut self, update: ActivityUpdate) {
        if let Self::Active { channel, .. } = self {
            // On error, turn itself into inactive to avoid sending unnecessary updates
            if channel.send(update).is_err() {
                *self = Self::Inactive;
            }
        }
    }

    /// Clear the Discord activity
    #[inline]
    pub fn clear_activity(&mut self) {
        self.update(ActivityUpdate::Clear);
        *self = Discord::Inactive;
    }

    /// Sets the current Discord activity to Main Menu
    #[inline]
    pub fn enter_main_menu(&mut self) { self.update(ActivityUpdate::MainMenu); }

    /// Sets the current Discord activity to Character Selection
    #[inline]
    pub fn enter_character_selection(&mut self) { self.update(ActivityUpdate::CharacterSelection); }

    /// Sets the current Discord activity to Singleplayer
    #[inline]
    pub fn join_singleplayer(&mut self) { self.update(ActivityUpdate::JoinSingleplayer); }

    /// Sets the current Discord activity to Multiplayer with the corresponding
    /// server name
    #[inline]
    pub fn join_server(&mut self, server_name: String) {
        self.update(ActivityUpdate::JoinServer(server_name));
    }

    /// Check the current location name and update it if it has changed
    #[inline]
    pub fn update_location(&mut self, chunk_name: &str, site: SiteKindMeta) {
        if let Self::Active {
            current_chunk_name,
            current_site,
            ..
        } = self
        {
            let different_name = current_chunk_name.as_deref() != Some(chunk_name);
            if different_name || *current_site != site {
                if different_name {
                    *current_chunk_name = Some(chunk_name.to_string());
                }
                *current_site = site;
                self.update(ActivityUpdate::NewLocation {
                    chunk_name: chunk_name.to_string(),
                    site,
                });
            }
        }
    }

    /// Check wether the Discord activity is active and receiving updates
    #[inline]
    pub fn is_active(&self) -> bool { matches!(self, Self::Active { .. }) }
}
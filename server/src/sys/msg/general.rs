use crate::client::Client;
use common::{
    comp::{Admin, ChatMode, ChatType, Content, Group, Player},
    event::{self, EmitExt},
    event_emitters,
    resources::ProgramTime,
    uid::Uid,
};
use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::{ClientGeneral, ServerGeneral};
use rayon::prelude::*;
use specs::{Entities, LendJoin, ParJoin, Read, ReadStorage, WriteStorage};
use tracing::{debug, error, warn};

event_emitters! {
    struct Events[Emitters] {
        command: event::CommandEvent,
        client_disconnect: event::ClientDisconnectEvent,
        chat: event::ChatEvent,

        #[cfg(feature = "plugins")]
        plugins: event::RequestPluginsEvent,
    }
}

impl Sys {
    fn handle_general_msg(
        emitters: &mut Emitters,
        entity: specs::Entity,
        client: &Client,
        player: Option<&Player>,
        uids: &ReadStorage<'_, Uid>,
        chat_modes: &ReadStorage<'_, ChatMode>,
        groups: &ReadStorage<'_, Group>,
        admins: &ReadStorage<'_, Admin>,
        msg: ClientGeneral,
    ) -> Result<(), crate::error::Error> {
        match msg {
            ClientGeneral::ChatMsg(message) => {
                if !client.client_type.can_send_message() {
                    client.send_fallible(ServerGeneral::ChatMsg(
                        ChatType::CommandError
                            .into_msg(Content::localized("command-cannot-send-message-hidden")),
                    ));
                } else if player.is_some() {
                    if let Some(from) = uids.get(entity) {
                        const CHAT_MODE_DEFAULT: &ChatMode = &ChatMode::default();
                        let mode = chat_modes.get(entity).unwrap_or(CHAT_MODE_DEFAULT);
                        // Try sending the chat message
                        match mode.to_msg(*from, message, groups.get(entity).copied()) {
                            Ok(message) => {
                                emitters.emit(event::ChatEvent {
                                    msg: message,
                                    from_client: true,
                                });
                            },
                            Err(error) => {
                                client.send_fallible(ServerGeneral::ChatMsg(
                                    ChatType::CommandError.into_msg(error),
                                ));
                            },
                        }
                    } else {
                        error!("Could not send message. Missing player uid");
                    }
                } else {
                    warn!("Received a chat message from an unregistered client");
                }
            },
            ClientGeneral::Command(name, args) => {
                if player.is_some() {
                    emitters.emit(event::CommandEvent(entity, name, args));
                }
            },
            ClientGeneral::FriendAction(action) => {
                if player.is_some() {
                    let (name, args) = match action {
                        common_net::msg::FriendAction::RequestList => {
                            ("friendlist".to_string(), Vec::new())
                        },
                        common_net::msg::FriendAction::Add(alias) => {
                            ("addfriend".to_string(), vec![alias])
                        },
                        common_net::msg::FriendAction::Accept(alias) => {
                            ("acceptfriend".to_string(), vec![alias])
                        },
                        common_net::msg::FriendAction::Reject(alias) => {
                            ("rejectfriend".to_string(), vec![alias])
                        },
                        common_net::msg::FriendAction::Remove(alias) => {
                            ("removefriend".to_string(), vec![alias])
                        },
                    };
                    emitters.emit(event::CommandEvent(entity, name, args));
                }
            },
            ClientGeneral::AdminAction(action) => {
                // Voxtera: admin actions are exclusively from the admin panel.
                // Server validates comp::Admin before applying each action.
                if player.is_some() {
                    let is_admin = admins.get(entity).is_some();
                    if !is_admin {
                        warn!(?entity, "Received admin action from non-admin player");
                    } else {
                        // Emit as a command event for the server tick to process
                        let action_str = format!("{:?}", action);
                        emitters.emit(event::CommandEvent(
                            entity,
                            "admin_action".to_string(),
                            vec![action_str],
                        ));
                    }
                }
            },
            ClientGeneral::Terminate => {
                debug!(?entity, "Client send message to terminate session");
                emitters.emit(event::ClientDisconnectEvent(
                    entity,
                    common::comp::DisconnectReason::ClientRequested,
                ));
            },
            ClientGeneral::RequestPlugins(plugins) => {
                tracing::info!("Plugin request {plugins:x?}, {}", player.is_some());

                #[cfg(feature = "plugins")]
                emitters.emit(event::RequestPluginsEvent { entity, plugins });
            },
            _ => {
                debug!("Kicking possible misbehaving client due to invalid message request");
                emitters.emit(event::ClientDisconnectEvent(
                    entity,
                    common::comp::DisconnectReason::NetworkError,
                ));
            },
        }
        Ok(())
    }
}

/// This system will handle new messages from clients
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Events<'a>,
        Read<'a, ProgramTime>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, ChatMode>,
        ReadStorage<'a, Player>,
        ReadStorage<'a, Group>,
        ReadStorage<'a, Admin>,
        WriteStorage<'a, Client>,
    );

    const NAME: &'static str = "msg::general";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (entities, events, program_time, uids, chat_modes, players, groups, admins, mut clients): Self::SystemData,
    ) {
        (&entities, &mut clients, players.maybe())
            .par_join()
            .for_each_init(
                || events.get_emitters(),
                |emitters, (entity, client, player)| {
                    let res = super::try_recv_all(client, 3, |client, msg| {
                        Self::handle_general_msg(
                            emitters,
                            entity,
                            client,
                            player,
                            &uids,
                            &chat_modes,
                            &groups,
                            &admins,
                            msg,
                        )
                    });

                    if let Ok(1_u64..=u64::MAX) = res {
                        // Update client ping.
                        client.last_ping = program_time.0
                    }
                },
            );
    }
}

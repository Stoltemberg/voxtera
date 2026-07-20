use crate::client::Client;
use common::comp::{ChatType, Content};
use common_net::msg::ServerGeneral;

/// Sends a welcome message to a player who just joined the server.
pub fn send_welcome_message(client: &Client, username: &str) {
    // Greeting
    client.send_fallible(ServerGeneral::server_msg(
        ChatType::Meta,
        Content::Plain(format!("Bem-vindo ao Voxtera, {}!", username)),
    ));

    // Controls tips
    client.send_fallible(ServerGeneral::server_msg(
        ChatType::Meta,
        Content::Plain(
            "Dicas de controlos: WASD para mover, Espaço para saltar, Shift para correr, Tab para \
             inventário, M para o mapa, T para conversar."
                .to_string(),
        ),
    ));
}

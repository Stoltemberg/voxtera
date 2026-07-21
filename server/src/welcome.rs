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

    // Controls tips — keys match voxygen/src/settings/control.rs defaults
    client.send_fallible(ServerGeneral::server_msg(
        ChatType::Meta,
        Content::Plain(
            "Controles: WASD mover | Espaço saltar | Shift agachar/descer | E interagir \
             | I inventário | C criação | P diário | M mapa | O amigos | Enter chat \
             | / comandos | F montar | G lanterna | H saudar | J dançar | K sentar."
                .to_string(),
        ),
    ));
}

## Player events, $user_gender should be available

hud-chat-online_msg = { "[" }{ $name }] está online.
hud-chat-offline_msg = { $name } ficou offline
hud-chat-goodbye = Até Logo!
hud-chat-connection_lost = Conexão perdida. Expulsando em { $time } segundos.

## Player /tell messages, $user_gender should be available

hud-chat-tell-to = Para [{ $alias }]: { $msg }
hud-chat-tell-from = Para [{ $alias }]: { $msg }

## Npc /tell messages, no gender info, sadly

hud-chat-tell-to-npc = Para [{ $alias }]: { $msg }
hud-chat-tell-from-npc = De [{ $alias }]: { $msg }

## Generic messages

hud-chat-message = { "[" }{ $alias }]: { $msg }
hud-chat-message-with-name = { "[" }{ $alias }] { $name }: { $msg }
hud-chat-message-in-group = ({ $group }) [{ $alias }]: { $msg }
hud-chat-message-in-group-with-name = ({ $group }) [{ $alias }] { $name }: { $msg }

## PvP Buff deaths, both $attacker_gender and $victim_gender are available

hud-chat-died_of_pvp_buff_msg = 
 .burning = [{ $victim }] died of: burning caused by [{ $attacker }]
 .bleeding = [{ $victim }] died of: bleeding caused by [{ $attacker }]
 .curse = [{ $victim }] died of: curse caused by [{ $attacker }]
 .crippled = [{ $victim }] died of: crippled caused by [{ $attacker }]
 .frozen = [{ $victim }] died of: frozen caused by [{ $attacker }]
 .mysterious = [{ $victim }] died of: secret caused by [{ $attacker }]

## PvE Buff deaths, only $victim_gender is available

hud-chat-died_of_npc_buff_msg = 
 .burning = [{ $victim }] died of: burning caused by { $attacker }
 .bleeding = [{ $victim }] died of: bleeding caused by { $attacker }
 .curse = [{ $victim }] died of: curse caused by { $attacker }
 .crippled = [{ $victim }] died of: crippled caused by { $attacker }
 .frozen = [{ $victim }] died of: frozen caused by { $attacker }
 .mysterious = [{ $victim }] died of: secret caused by { $attacker }

## Random Buff deaths, only $victim_gender is available

hud-chat-died_of_buff_nonexistent_msg = 
 .burning = [{ $victim }] died of: burning
 .bleeding = [{ $victim }] died of: bleeding
 .curse = [{ $victim }] died of: curse
 .crippled = [{ $victim }] died of: crippled
 .frozen = [{ $victim }] died of: frozen
 .mysterious = [{ $victim }] died of: secret

## Other PvP deaths, both $attacker_gender and $victim_gender are available

hud-chat-pvp_melee_kill_msg = { "[" }{ $attacker }] derrotou [{ $victim }]
hud-chat-pvp_ranged_kill_msg = { "[" }{ $attacker }] atirou em [{ $victim }]
hud-chat-pvp_explosion_kill_msg = { "[" }{ $attacker }] explodiu [{ $victim }]
hud-chat-pvp_energy_kill_msg = { "[" }{ $attacker }] matou [{ $victim }] com magia
hud-chat-pvp_other_kill_msg = { "[" }{ $attacker }] matou [{ $victim }]

## Other PvE deaths, only $victim_gender is available

hud-chat-npc_melee_kill_msg = { $attacker } matou [{ $victim }]
hud-chat-npc_ranged_kill_msg = { $attacker } atirou em [{ $victim }]
hud-chat-npc_explosion_kill_msg = { $attacker } explodiu [{ $victim }]
hud-chat-npc_energy_kill_msg = { "[" }{ $attacker }] matou [{ $victim }] com magia
hud-chat-npc_other_kill_msg = { "[" }{ $attacker }] matou [{ $victim }]

## Other deaths, only $victim_gender is available

hud-chat-fall_kill_msg = { "[" }{ $name }] morreu de dano de queda
hud-chat-suicide_msg = { "[" }{ $name }] morreu de dano autoinflingido
hud-chat-default_death_msg = { "[" }{ $name }] morreu

## Chat utils

hud-chat-all = Todos
hud-chat-chat_tab_hover_tooltip = Clique direito para configurar

## HUD Pickup message

hud-loot-pickup-msg-you = { $amount ->
    [1] You picked up { $item }
    *[other] You picked up {$amount}x {$item}
}
hud-loot-pickup-msg = { $amount ->
    [1] { $actor } picked up { $item }
    *[other] { $actor } picked up { $amount }x { $item }
}

hud-chat-singleplayer-motd1 = Um mundo inteiro só seu! Hora de espreguiçar...
hud-chat-singleplayer-motd2 = Como está a serenidade?

# Mensagens de sistema sobre comandos do chat. Acompanha o idioma selecionado pelo jogador
# nas configurações do jogo, independentemente da localidade do servidor.
hud-chat-meta-group-join-hint = Digite /g ou /group para conversar com os membros do seu grupo.
hud-chat-meta-group-joined = [{ $alias }] entrou no grupo.
hud-chat-meta-group-left = [{ $alias }] saiu do grupo.
hud-chat-meta-group-invite-already-member = Falha no convite: esse jogador já está no seu grupo.
hud-chat-meta-group-invite-full = Falha no convite: seu grupo está cheio ({ $max } jogadores).
hud-chat-meta-invite-target-missing = Falha no convite: o alvo não existe.
hud-chat-meta-invite-pending = Esse jogador já possui um convite pendente.
hud-chat-meta-invite-not-player = Não é possível convidar: não é um jogador ou NPC.
hud-chat-meta-group-kick-target-missing = Falha na expulsão: o alvo não existe.
hud-chat-meta-group-kick-pet = Falha na expulsão: você não pode expulsar pets.
hud-chat-meta-group-kick-self = Falha na expulsão: você não pode expulsar a si mesmo.
hud-chat-meta-group-kick-not-leader = Falha na expulsão: você não é o líder do grupo do alvo.
hud-chat-meta-group-kick-target-not-in-group = Falha na expulsão: seu alvo não está em um grupo.
hud-chat-meta-group-removed = Você foi removido do grupo.
hud-chat-meta-group-kick-success = Jogador expulso.
hud-chat-meta-group-leader-now = Você agora é o líder do grupo.
hud-chat-meta-group-leader-no-longer = Você não é mais o líder do grupo.
hud-chat-meta-group-leader-transfer-target-missing = Falha na transferência de liderança: o alvo não existe.
hud-chat-meta-group-leader-transfer-not-leader = Falha na transferência: você não é o líder do grupo do alvo.
hud-chat-meta-group-leader-transfer-target-not-in-group = Falha na transferência: seu alvo não está em um grupo.
hud-chat-meta-trade-inviter-new-trade = Falha na troca: o convidante iniciou uma nova troca desde o envio do pedido.
hud-chat-meta-faction-joined = [{ $alias }] entrou na facção ({ $faction }).
hud-chat-meta-faction-left = [{ $alias }] saiu da facção ({ $faction }).
hud-chat-meta-screenshot-taken = Captura de tela salva em { $path }.
hud-chat-meta-screenshot-failed = Não foi possível salvar a captura de tela.
hud-chat-meta-screenshot-folder-failed = Não foi possível criar a pasta para capturas de tela.
hud-chat-meta-screenshot-error = Erro ao gerar captura de tela: { $error }.
hud-chat-meta-server-saved = Estado do servidor salvo.
hud-chat-meta-time-too-long = O valor de tempo selecionado é muito grande ou muito pequeno.
hud-chat-meta-not-a-positive-number = Não é um número positivo.
hud-chat-meta-not-an-integer = Não é um número inteiro.
## Player events, $user_gender should be available

hud-chat-online_msg = [{ $name }] is online now.
hud-chat-offline_msg = [{ $name }] went offline.
hud-chat-goodbye = Goodbye!
hud-chat-connection_lost = Connection lost. Kicking in { $time } seconds.

## Player /tell messages, $user_gender should be available

hud-chat-tell-to = To [{ $alias }]: { $msg }
hud-chat-tell-from = From [{ $alias }]: { $msg }

## Npc /tell messages, no gender info, sadly

hud-chat-tell-to-npc = To [{ $alias }]: { $msg }
hud-chat-tell-from-npc = From [{ $alias }]: { $msg }

## Generic messages

hud-chat-message = [{ $alias }]: { $msg }
hud-chat-message-with-name = [{ $alias }] { $name }: { $msg }
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

hud-chat-pvp_melee_kill_msg = [{ $attacker }] defeated [{ $victim }]
hud-chat-pvp_ranged_kill_msg = [{ $attacker }] shot [{ $victim }]
hud-chat-pvp_explosion_kill_msg = [{ $attacker }] blew up [{ $victim }]
hud-chat-pvp_energy_kill_msg = [{ $attacker }] killed [{ $victim }] with magic
hud-chat-pvp_other_kill_msg = [{ $attacker }] killed [{ $victim }]

## Other PvE deaths, only $victim_gender is available

hud-chat-npc_melee_kill_msg = { $attacker } killed [{ $victim }]
hud-chat-npc_ranged_kill_msg = { $attacker } shot [{ $victim }]
hud-chat-npc_explosion_kill_msg = { $attacker } blew up [{ $victim }]
hud-chat-npc_energy_kill_msg = { $attacker } killed [{ $victim }] with magic
hud-chat-npc_other_kill_msg = { $attacker } killed [{ $victim }]

## Other deaths, only $victim_gender is available

hud-chat-fall_kill_msg = [{ $name }] died from fall damage
hud-chat-suicide_msg = [{ $name }] died from self-inflicted wounds
hud-chat-default_death_msg = [{ $name }] died

## Chat utils

hud-chat-all = All
hud-chat-chat_tab_hover_tooltip = Right click for settings

## HUD Pickup message

hud-loot-pickup-msg-you = { $amount ->
    [1] You picked up { $item }
    *[other] You picked up {$amount}x {$item}
}
hud-loot-pickup-msg = { $amount ->
    [1] { $actor } picked up { $item }
    *[other] { $actor } picked up { $amount }x { $item }
}

hud-chat-singleplayer-motd1 = A whole world to yourself! Time to stretch...
hud-chat-singleplayer-motd2 = How's the serenity?

# Meta/system messages about chat commands. These follow the language the player
# selected in the game settings, regardless of the server locale.
hud-chat-meta-group-join-hint = Type /g or /group to chat with your group members.
hud-chat-meta-group-joined = [{ $alias }] joined the group.
hud-chat-meta-group-left = [{ $alias }] left the group.
hud-chat-meta-faction-joined = [{ $alias }] joined faction ({ $faction }).
hud-chat-meta-faction-left = [{ $alias }] left faction ({ $faction }).
hud-chat-meta-screenshot-taken = Screenshot saved to { $path }.
hud-chat-meta-screenshot-failed = Couldn't save screenshot.
hud-chat-meta-screenshot-folder-failed = Couldn't create folder for screenshot.
hud-chat-meta-screenshot-error = Error when generating screenshot: { $error }.
hud-chat-meta-server-saved = Server state saved.
hud-chat-meta-time-too-long = Selected time value is too large or too small.
hud-chat-meta-not-a-positive-number = Not a positive number.
hud-chat-meta-not-an-integer = Not an integer.
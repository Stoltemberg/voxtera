# Descriptions and Help

command-help-template = { $usage } { $description }
command-help-list = 
  { $client-commands }
  { $server-commands }

  Além disso, você pode usar os seguintes atalhos:
  { $additional-shortcuts }



## Server Commands

command-adminify-desc = Concede temporariamente a um jogador um cargo de admin restrito ou remove o atual (se não fornecido)
command-airship-desc = Spawna uma nave
command-alias-desc = Mude seu apelido
command-area_add-desc = Adiciona uma nova área de construção
command-area_list-desc = Lista todas as áreas de construção
command-area_remove-desc = Remove a área de construção especificada
command-aura-desc = Cria uma aura
command-body-desc = Mude seu corpo para uma espécie diferente
command-set_body_type-desc = Defina seu tipo de corpo, Feminino ou Masculino.
command-set_body_type-not_found = Esse não é um tipo de corpo válido.
  Tente um dos seguintes:
  { $options }
command-set_body_type-no_body = Não foi possível definir o tipo de corpo pois o alvo não tem um corpo.
command-set_body_type-not_character = Só é possível definir permanentemente o tipo de corpo se o alvo for um jogador online como personagem.
command-buff-desc = Aplica um buff no jogador
command-build-desc = Ativa/desativa o modo construção
command-ban-desc = Bane um jogador com um nome de usuário, por uma duração (se fornecida). Passe true para sobrescrever um ban existente.
command-ban-ip-desc = Bane um jogador com um nome de usuário, por uma duração (se fornecida). Diferentemente do ban normal, isso também bane o endereço de IP associado a este usuário. Passe true para sobrescrever um ban existente.
command-battlemode-desc = Define seu modo de batalha para:
  + pvp (jogador vs jogador)
  + pve (jogador vs ambiente).
  Se chamado sem argumentos mostra o modo de batalha atual.
command-battlemode_force-desc = Muda sua flag de modo de batalha sem verificações
command-campfire-desc = Spawna uma fogueira
command-clear_persisted_terrain-desc = Limpa o terreno persistido próximo
command-create_location-desc = Cria uma localização na posição atual
command-death_effect-dest = Adiciona um efeito de morte à entidade alvo
command-debug_column-desc = Imprime informações de debug sobre uma coluna
command-debug_ways-desc = Imprime informações de debug sobre as formas de uma coluna
command-delete_location-desc = Deleta uma localização
command-destroy_tethers-desc = Destrói todas as conexões conectadas a você
command-disconnect_all_players-desc = Desconecta todos os jogadores do servidor
command-dismount-desc = Desmonta se você estiver montando, ou desmonta qualquer coisa que estiver te montando
command-dropall-desc = Joga todos os seus itens no chão
command-dummy-desc = Spawna um boneco de treino
command-explosion-desc = Explode o chão ao seu redor
command-faction-desc = Envia mensagens para sua facção
command-give_item-desc = Dá a si mesmo alguns itens. Para um exemplo ou auto completar use Tab.
command-gizmos-desc = Gerencia assinaturas de gizmo.
command-gizmos_range-desc = Muda o alcance das assinaturas de gizmo.
command-goto-desc = Teletransporta para uma posição
command-goto-rand = Teletransporta para uma posição aleatória
command-group-desc = Envia mensagens para seu grupo
command-group_invite-desc = Convida um jogador para entrar em um grupo
command-group_kick-desc = Remove um jogador de um grupo
command-group_leave-desc = Deixa o grupo atual
command-group_promote-desc = Promove um jogador a líder do grupo
command-health-desc = Define sua saúde atual
command-into_npc-desc = Converte você em um NPC. Cuidado!
command-join_faction-desc = Entra/sai da facção especificada
command-jump-desc = Move sua posição atual
command-kick-desc = Expulsa um jogador com um nome de usuário
command-kill-desc = Se mata
command-kill_npcs-desc = Mata os NPCs
command-kit-desc = Coloca um conjunto de itens no seu inventário.
command-lantern-desc = Muda a força e cor da sua lanterna
command-light-desc = Spawna entidade com luz
command-lightning-desc = Raio na posição atual
command-location-desc = Teletransporta para uma localização
command-make_block-desc = Faz um bloco na sua posição com uma cor
command-make_npc-desc = Spawna entidade de config perto de você.
  Para um exemplo ou auto completar use Tab.
command-make_sprite-desc = Faz um sprite na sua posição, para definir atributos do sprite use sintaxe ron para um StructureSprite.
command-make_volume-desc = Cria um volume (experimental)
command-motd-desc = Vê a descrição do servidor
command-mount-desc = Monta uma entidade
command-object-desc = Spawna um objeto
command-outcome-desc = Cria um resultado
command-permit_build-desc = Concede a um jogador uma caixa delimitada onde pode construir
command-players-desc = Lista jogadores atualmente online
command-poise-desc = Define sua postura atual
command-portal-desc = Spawna um portal
command-region-desc = Envia mensagens para todos na sua região do mundo
command-reload_chunks-desc = Recarrega chunks carregados no servidor
command-remove_lights-desc = Remove todas as luzes spawnadas por jogadores
command-repair_equipment-desc = Repara todos os itens equipados
command-reset_recipes-desc = Reseta seu livro de receitas
command-respawn-desc = Teletransporta para seu waypoint
command-revoke_build-desc = Revoga permissão de construção do jogador
command-revoke_build_all-desc = Revoga todas as permissões de construção do jogador
command-safezone-desc = Cria uma zona segura
command-say-desc = Envia mensagens para todos dentro de distância de grito
command-scale-desc = Escala seu personagem
command-server_physics-desc = Define/desdefine física autoritativa do servidor para uma conta
command-set_motd-desc = Define a descrição do servidor
command-set-waypoint-desc = Define seu waypoint na sua localização atual.
command-ship-desc = Spawna um barco
command-site-desc = Teletransporta para um local
command-skill_point-desc = Dá a si mesmo pontos de habilidade para uma árvore de habilidades específica
command-skill_preset-desc = Dá ao seu personagem as habilidades desejadas.
command-spawn-desc = Spawna uma entidade de teste
command-spot-desc = Encontra e teletransporta para o local mais próximo de um certo tipo.
command-sudo-desc = Executa comando como se fosse outra entidade
command-tell-desc = Envia mensagem para outro jogador
command-tether-desc = Conecta outra entidade a você
command-time-desc = Define a hora do dia
command-time_scale-desc = Define a escala do tempo delta
command-tp-desc = Teletransporta para outra entidade
command-rtsim_chunk-desc = Mostra informações sobre o chunk atual do rtsim
command-rtsim_info-desc = Mostra informações sobre um NPC do rtsim
command-rtsim_npc-desc = Lista NPCs do rtsim que correspondem a uma query (ex: simulated,merchant) em ordem de distância
command-rtsim_purge-desc = Purgua dados do rtsim na próxima inicialização
command-rtsim_tp-desc = Teletransporta para um NPC do rtsim
command-unban-desc = Remove o ban do nome de usuário fornecido. Se houver um ban de IP associado ele também será removido.
command-unban-ip-desc = Remove apenas o ban de IP do nome de usuário fornecido.
command-version-desc = Imprime versão do servidor
command-weather_zone-desc = Cria uma zona de clima
command-whitelist-desc = Adiciona/remove nome de usuário da whitelist
command-wiring-desc = Cria elemento de fiação
command-world-desc = Envia mensagens para todos no servidor
command-wiki-desc = Abre a wiki ou pesquisa um tópico
command-reset_tutorial-desc = Reseta o tutorial do jogo para seu estado inicial
command-reset_tutorial-success = Tutorial resetado.
command-naga-desc = Ativa/desativa uso do naga no processamento inicial de shader (não persistido)
# Command: /players
players-list-header = { $count ->
  [1] { $count } jogador online
    { $player_list }
  *[other] { $count } jogadores online
    { $player_list }
}
## Voxygen Client Commands

command-clear-desc = Limpa todas as mensagens no chat. Afeta todas as abas de chat.
command-experimental_shader-desc = Ativa um shader experimental.
command-help-desc = Mostra informações sobre comandos
command-mute-desc = Silencia mensagens de chat de um jogador.
command-unmute-desc = Desativa o silêncio de um jogador silenciado com o comando mute.
command-waypoint-desc = Mostra a localização do waypoint atual
command-preprocess-target-error = Esperado { $expected_list } após "@" encontrado { $target }
command-preprocess-not-looking-at-valid-target = Não está olhando para um alvo válido
command-preprocess-not-selected-valid-target = Não selecionou um alvo válido
command-preprocess-not-valid-viewpoint-entity = Não está viewando de uma entidade viewpoint válida
command-preprocess-not-riding-valid-entity = Não está montando uma entidade válida
command-preprocess-not-valid-rider = Sem cavaleiro válido
command-preprocess-no-player-entity = Sem entidade de jogador
command-invalid-command-message = 
  Não foi possível encontrar um comando chamado { $invalid-command }.
  Você quis dizer algum dos seguintes?
  { $most-similar-command }
  { $commands-with-same-prefix }

  Digite /help para ver a lista de todos os comandos.


command-mute-cannot-mute-self = Você não pode se silenciar
command-mute-success = Jogador { $player } silenciado com sucesso
command-mute-no-player-found = Não foi possível encontrar jogador chamado { $player }
command-mute-already-muted = { $player } já está silenciado
command-mute-no-player-specified = Você deve especificar um jogador
command-unmute-cannot-unmute-self = Você não pode se des-silenciar
command-unmute-success = Jogador { $player } des-silenciado com sucesso
command-unmute-no-muted-player-found = Não foi possível encontrar jogador silenciado chamado { $player }
command-unmute-no-player-specified = Você deve especificar um jogador para silenciar
command-shader-backend = Shader Backend Atual: { $shader-backend }
# Only returns a list of shaders
command-experimental-shaders-list = { $shader-list }
command-experimental-shaders-not-found = Não há shaders experimentais
command-experimental-shaders-enabled = Ativado { $shader }
command-experimental-shaders-disabled = Desativado { $shader }
command-experimental-shaders-not-supported = { $shader } não é suportado por esta build do jogo
command-experimental-shaders-not-a-shader = { $shader } não é um shader experimental, use este comando sem argumentos para ver a lista completa.
command-experimental-shaders-not-valid = Você deve especificar um shader experimental válido, para obter uma lista de shaders experimentais use este comando sem argumentos.

# Results and Warning

command-no-permission = Você não tem permissão para usar '/{ $command_name }'
command-position-unavailable = Não foi possível obter posição de { $target }
command-player-role-unavailable = Não foi possível obter cargos de administrador para { $target }
command-uid-unavailable = Não foi possível obter UID para { $target }
command-area-not-found = Não foi possível encontrar área chamada '{ $area }'
command-player-not-found = Jogador '{ $player }' não encontrado!
command-player-uuid-not-found = Jogador com UUID '{ $uuid }' não encontrado!
command-username-uuid-unavailable = Não foi possível determinar UUID para nome de usuário { $username }
command-uuid-username-unavailable = Não foi possível determinar nome de usuário para UUID { $uuid }
command-no-sudo = É rude se passar por outras pessoas
command-entity-dead = Entidade '{ $entity }' está morta!
command-error-write-settings = Falha ao escrever arquivo de configurações no disco, mas succeeded na memória.
  Erro (storage): { $error }
  Sucesso (memória): { $message }
command-error-while-evaluating-request = Erro encontrado ao validar a requisição: { $error }
command-give-inventory-full = Inventário do jogador cheio. Deu apenas { $given ->
  [1] apenas um
  *[other] { $given }
} de { $total } itens.
command-give-inventory-success = Adicionados { $total } x { $item } ao inventário.
command-invalid-item = Item inválido: { $item }
command-invalid-block-kind = Tipo de bloco inválido: { $kind }
command-nof-entities-at-least = Número de entidades deve ser pelo menos 1
command-nof-entities-less-than = Número de entidades deve ser menor que 50
command-entity-load-failed = Falha ao carregar config de entidade: { $config }
command-spawned-entities-config = Spawnadas { $n } entidades do config: { $config }
command-invalid-sprite = Tipo de sprite inválido: { $kind }
command-time-parse-too-large = { $n } é inválido, não pode ser maior que 16 dígitos.
command-time-parse-negative = { $n } é inválido, não pode ser negativo.
command-time-backwards = { $t } é antes do tempo atual, tempo não pode voltar.
command-time-invalid = { $t } não é um tempo válido.
command-time-current = Agora é { $t }
command-time-unknown = Tempo desconhecido
command-rtsim-purge-perms = Você precisa ser um admin real (não apenas um admin temporário) para purgar dados do rtsim.
command-chunk-not-loaded = Chunk { $x }, { $y } não carregado
command-chunk-out-of-bounds = Chunk { $x }, { $y } fora dos limites do mapa
command-spawned-entity = Entidade spawnada com ID: { $id }
command-spawned-dummy = Spawnado um boneco de treino
command-spawned-airship = Spawnada uma nave
command-spawned-campfire = Spawnada uma fogueira
command-spawned-safezone = Spawnada uma zona segura
command-volume-size-incorrect = Tamanho precisa estar entre 1 e 127.
command-volume-created = Volume criado
command-permit-build-given = Agora você tem permissão para construir em '{ $area }'
command-permit-build-granted = Permissão para construir em '{ $area }' concedida
command-revoke-build-recv = Sua permissão para construir em '{ $area }' foi revogada
command-revoke-build = Permissão para construir em '{ $area }' revogada
command-revoke-build-all = Suas permissões de construção foram revogadas.
command-revoked-all-build = Todas as permissões de construção revogadas.
command-no-buid-perms = Você não tem permissão para construir.
command-set-build-mode-off = Modo construção desativado.
command-set-build-mode-on-persistent = Modo construção ativado. Persistência experimental de terreno está habilitada. O servidor tentará persistir mudanças, mas isso não é garantido.
command-set-build-mode-on-unpersistent = Modo construção ativado. Mudanças não serão persistidas quando um chunk descarregar.
command-set_motd-message-added = Mensagem do dia do servidor definida como { $message }
command-set_motd-message-removed = Mensagem do dia do servidor removida
command-set_motd-message-not-set = Esta locale não tinha motd definido
command-set-waypoint-result = Waypoint definido!
command-invalid-alignment = Alinhamento inválido: { $alignment }
command-kit-not-enough-slots = Inventário não tem slots suficientes
command-lantern-unequiped = Por favor equipe uma lanterna primeiro
command-lantern-adjusted-strength = Você ajustou a força da chama.
command-lantern-adjusted-strength-color = Você ajustou a força e cor da chama.
command-explosion-power-too-high = Poder da explosão não pode ser mais que { $power }
command-explosion-power-too-low = Poder da explosão deve ser mais que { $power }
# Note: Do not translate "confirm" here
command-disconnectall-confirm = Por favor execute o comando novamente com o segundo argumento "confirm" para confirmar que
  você realmente quer desconectar todos os jogadores do servidor
command-invalid-skill-group = { $group } não é um grupo de habilidade!
command-unknown = Comando desconhecido
command-disabled-by-settings = Comando desabilitado nas configurações do servidor
command-battlemode-intown = Você precisa estar na cidade para mudar o modo de batalha!
command-battlemode-cooldown = Período de cooldown ativo. Tente novamente em { $cooldown } segundos
command-battlemode-available-modes = Modos disponíveis: pvp, pve
command-battlemode-same = Tentou definir o mesmo modo de batalha
command-battlemode-updated = Novo modo de batalha: { $battlemode }
command-buff-unknown = Buff desconhecido: { $buff }
command-buff-data = Argumento de buff "{ $buff }" requer dados adicionais
command-buff-body-unknown = Spec de corpo desconhecido: { $spec }
command-skillpreset-load-error = Erro ao carregar presets
command-skillpreset-broken = Preset de habilidade está quebrado
command-skillpreset-missing = Preset não existe: { $preset }
command-location-invalid = Nome de localização '{ $location }' inválido. Nomes devem conter apenas ASCII minúsculo e
  sublinhados
command-location-duplicate = Localização '{ $location }' já existe, considere deletá-la primeiro
command-location-not-found = Localização '{ $location }' não existe
command-location-created = Localização '{ $location }' criada
command-location-deleted = Localização '{ $location }' deletada
command-locations-empty = Nenhuma localização existe atualmente
command-locations-list = Localizações disponíveis: { $locations }
# Note: Do not translate these weather names
command-weather-valid-values = Valores válidos são 'clear', 'rain', 'wind' e 'storm'.
command-scale-set = Escala definida para { $scale }
command-repaired-items = Todos os itens equipados reparados
command-repaired-inventory_items = Todos os itens reparados
command-message-group-missing = Você está usando chat de grupo mas não pertence a um grupo. Use /world ou
  /region para mudar o chat.
command-tell-to-yourself = Você não pode /tell a si mesmo.
command-transform-invalid-presence = Não é possível transformar na presença atual
command-aura-invalid-buff-parameters = Parâmetros de buff inválidos para aura
command-aura-spawn = Nova aura spawnada anexada à entidade
command-aura-spawn-new-entity = Nova aura spawnada
command-reloaded-chunks = { $reloaded } chunks recarregados
command-server-no-experimental-terrain-persistence = Servidor foi compilado sem persistência de terreno habilitada
command-experimental-terrain-persistence-disabled = Persistência experimental de terreno está desabilitada
command-adminify-assign-higher-than-own = Não é possível atribuir a alguém um cargo temporário maior que seu próprio cargo permanente.
command-adminify-reassign-to-above = Não é possível realocar um cargo para alguém com seu cargo ou maior.
command-adminify-cannot-find-player = Não foi possível encontrar entidade do jogador!
command-adminify-already-has-role = Jogador já tem esse cargo!
command-adminify-already-has-no-role = Jogador já não tem cargo!
command-adminify-role-downgraded = Cargo do jogador { $player } rebaixado para { $role }
command-adminify-role-upgraded = Cargo do jogador { $player } promovido para { $role }
command-adminify-removed-role = Cargo removido do jogador { $player }: { $role }
command-ban-added = { $player } adicionado à lista de bans com reason: { $reason }
command-ban-already-added = { $player } já está na lista de bans
command-ban-ip-added = { $player } adicionado à lista de bans regular e lista de bans de IP com reason: { $reason }
command-ban-ip-queued = { $player } adicionado à lista de bans regular e ban de IP agendado com reason: { $reason }
command-faction-join = Por favor entre em uma facção com /join_faction
command-group-join = Por favor crie um grupo primeiro
command-group_invite-invited-to-group = { $player } convidado para o grupo.
command-group_invite-invited-to-your-group = { $player } foi convidado para seu grupo.
command-into_npc-warning = Espero que não esteja abusando disso!
command-kick-higher-role = Não é possível expulsar jogadores com cargos maiores que o seu.
command-respawn-no-waypoint = Nenhum waypoint definido
command-site-not-found = Site não encontrado
command-sudo-higher-role = Não é possível sudo em jogadores com cargos maiores que o seu.
command-sudo-no-permission-for-non-players = Você não tem permissão para dar sudo em não-jogadores.
command-time_scale-current = A escala de tempo atual é { $scale }.
command-time_scale-changed = Escala de tempo definida para { $scale }.
command-unban-successful = { $player } foi desbanido com sucesso.
command-unban-ip-successful = O ban de IP via usuário "{ $player }" foi desbanido com sucesso (este usuário permanecerá banido)
command-unban-already-unbanned = { $player } já estava desbanido.
command-version-current = Servidor está rodando { $version }
command-whitelist-added = Adicionado à whitelist: { $username }
command-whitelist-already-added = Já está na whitelist: { $username }!
command-whitelist-removed = Removido da whitelist: { $username }
command-whitelist-unlisted = Não faz parte da whitelist: { $username }
command-whitelist-permission-denied = Permissão negada para remover usuário: { $username }
command-outcome-variant_expected = Variante de resultado esperada
command-outcome-expected_body_arg = Esperado argumento de corpo
command-outcome-expected_entity_arg = Esperado argumento de entidade
command-outcome-expected_skill_group_kind = Esperado ron SkillGroupKind válido
command-outcome-expected_frontent_specifier = Esperado especificador de frontent
command-outcome-expected_integer = Esperado inteiro
command-outcome-expected_sprite_kind = Esperado SpriteKind
command-outcome-invalid_outcome = { $outcome } não é um resultado válido
command-death_effect-unknown = Efeito de morte desconhecido { $effect }.
command-spot-spot_not_found = Não encontrou nenhum local desse tipo neste mundo.
command-spot-world_feature = O recurso "worldgen" precisa estar habilitado para executar este comando.
command-cannot-send-message-hidden = Não é possível enviar mensagens como espectador escondido.
command-destroyed-tethers = Todas as conexões destruídas! Você está livre agora
command-destroyed-no-tethers = Você não está conectado a nenhuma conexão
command-dismounted = Desmontado
command-no-dismount = Você não está montando ou sendo montado
command-client-has-no-socketaddr = Não é possível obter endereço de soquete (conectado via conexão mpsc) para { $target }
command-parse-duration-error = Não foi possível analisar duração: { $error }
command-waypoint-result = Seu waypoint atual está em { $waypoint };
command-waypoint-error = Não foi possível encontrar seu waypoint.

# Unreachable/untestable but added for consistency

command-player-info-unavailable = Não foi possível obter informação do jogador para { $target }
command-unimplemented-spawn-special = Spawnar entidades especiais não está implementado
command-kit-inventory-unavailable = Não foi possível obter inventário
command-inventory-cant-fit-item = Não cabe item no inventário
# Emitted by /disconnect_all when you don't exist (?)
command-you-dont-exist = Você não existe, então não pode usar este comando
command-entity-has-no-client = Jogador não tem componente de cliente: { $target }
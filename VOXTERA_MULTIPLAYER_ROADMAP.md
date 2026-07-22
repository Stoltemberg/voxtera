# Voxtera — Roadmap de Melhorias Multiplayer

> Criado em 2026-07-22. Release atual: **v0.2.5**.
> Cada fase termina com `cargo check --workspace` + build release + publicação no GitHub.

---

## FASE 1 — Fundações Admin & Social (v0.2.6)
> Objetivo: dar ao host ferramentas práticas e melhorar a vida em grupo.

### 1.1 Painel Admin Secreto (Ctrl+Alt+F12)
- **Atalho:** `Ctrl+Alt+F12` abre/fecha painel admin invisível para jogadores comuns
- **Widget:** novo `voxygen/src/hud/admin_panel.rs` (Conrod)
- **Features do painel:**
  - Lista de jogadores online (com UUID e IP)
  - Botões: Kick, Mute, Teleportar para jogador, Trazer jogador
  - Campo de announce integrado (sem precisar digitar comando)
  - Toggle PvP global
  - Gerar item (selecionar + quantidade)
  - Ver logs de auditoria em tempo real
- **Segurança:** painel só aparece se `comp::Admin` estiver no jogador
- **i18n:** PT-BR + EN em `hud/admin.ftl`
- **Arquivos novos:** `hud/admin_panel.rs`, `i18n/*/hud/admin.ftl`
- **Arquivos modificados:** `hud/mod.rs` (widget ID + keybinding), `settings/control.rs` (novo `GameInput::AdminPanel`)

### 1.2 Ações Admin (Exclusivas do Painel — Sem Comandos de Chat)
> **Importante:** Nenhuma ação admin é acessível via chat. Todos os comandos
> (`/tp`, `/give`, `/kick`, etc.) existem APENAS como botões/inputs no painel
> admin (Ctrl+Alt+F12). Jogadores comuns não podem invocá-los.

- **Teleportar** — botão "TP Para" e "TP Aqui" na lista de jogadores online
- **Gerar Item** — dropdown de item + campo de quantidade
- **Kick** — botão "Expulsar" ao lado de cada jogador
- **Mute** — botão "Silenciar" com seletor de duração
- **Unmute** — botão "Restaurar Voz"
- **God Mode** — toggle no painel
- **Protocolo:** novo enum `AdminAction` em `common/net/src/msg/client.rs`
  - `AdminAction::TeleportTo(Uid)`, `AdminAction::TeleportHere(Uid)`
  - `AdminAction::GiveItem(String, u16)`, `AdminAction::Kick(Uid)`
  - `AdminAction::Mute(Uid, Duration)`, `AdminAction::Unmute(Uid)`
  - `AdminAction::GodMode`, `AdminAction::TogglePvp`
- **Segurança:** servidor valida `comp::Admin` em cada `AdminAction` recebida
- **Arquivos novos:** enum em `common/net/src/msg/client.rs`, handlers em `server/src/sys/msg/general.rs`
- **Arquivos modificados:** `server/src/lib.rs` (não process_command — novo path via msg)

### 1.3 Logs de Auditoria
- Registrar em `server/data_dir/audit.log`:
  - Ações admin (kick, mute, give, tp, announce)
  - Kills PvP (killer, victim, weapon)
  - Trades (player A ↔ player B, items trocados)
  - Logins/logouts (UUID, alias, IP, timestamp)
- **Arquivo novo:** `server/src/audit_log.rs`
- **Arquivos modificados:** `server/src/lib.rs` (insert resource + tick flush)

### 1.4 Chat de Grupo (Party Chat)
- Novo canal de chat: mensagens visíveis apenas para membros do grupo
- **Comando:** `/p <mensagem>` ou `/party <mensagem>`
- **Cliente:** tab de chat "Grupo" com cor distinta
- **Arquivos:** `server/src/chat.rs`, `server/src/lib.rs` (dispatch), `voxygen/src/hud/chat.rs`, `voxygen/src/settings/chat.rs`

---

## FASE 2 — Experiência de Grupo (v0.2.7)
> Objetivo: fazer jogar em grupo ser mais divertido e informativo.

### 2.1 Barra de HP do Grupo no HUD
- Mostrar retratos + barras de vida/energia dos membros do party
- Cores dinâmicas (verde → amarelo → vermelho)
- Destacar membro sob ataque (flash vermelho)
- **Arquivo:** `voxygen/src/hud/group.rs` (expandir widget existente)

### 2.2 Minimapa com Marcadores de Amigos
- Pontos verdes no minimapa para amigos online dentro do raio de view distance
- Pontos azuis para membros do grupo
- **Arquivo:** `voxygen/src/hud/minimap.rs`

### 2.3 Notificações de Loot Raro
- Popup visual quando item de qualidade High+ dropar
- Som de notificação + texto flutuante com nome e qualidade colorida
- **Arquivos:** `voxygen/src/hud/loot_scroller.rs`, `voxygen/src/hud/popup.rs`

### 2.4 Sistema de Bloqueio (/block)
- `/block <player>` — bloqueia mensagens privadas e convites
- `/unblock <player>`
- Lista de bloqueados no painel de amigos
- **Arquivos:** `server/src/friends.rs` (estender), `server/src/lib.rs` (dispatch), `voxygen/src/hud/friends_panel.rs`

---

## FASE 3 — Gameplay & Economia (v0.2.8)
> Objetivo: dar propósito e progressão ao multiplayer.

### 3.1 Lojas de NPC
- NPCs selecionados compram/vendem itens por moeda
- Interface de trade simplificada (comprar item → deduz moeda)
- Inventario da loja regenera ao longo do tempo
- **Arquivos novos:** `server/src/shop.rs`, `voxygen/src/hud/shop.rs`
- **Arquivos modificados:** `server/src/events/trade.rs`, `server/src/events/interaction.rs`

### 3.2 Modo PvP Opcional
- `/pvp on/off` — toggle individual
- Jogadores com PvP on podem atacar e ser atacados por outros PvP on
- Jogadores com PvP off são imunes a dano de outros jogadores
- Indicador visual: borda vermelha no portrait se PvP on
- **Arquivos:** `common/src/comp/` (novo `PvpFlag`), `server/src/events/entity_manipulation.rs`, `server/src/lib.rs`

### 3.3 XP Flutuante & Animação de Level-Up
- Números de XP sobem do inimigo morto até a barra de skill
- Flash dourado + texto "Level X!" ao subir de nível
- **Arquivos:** `voxygen/src/hud/skillbar.rs`, `voxygen/src/hud/animation.rs`

### 3.4 Tutorial Contextual
- Dicas que aparecem baseadas no contexto:
  - Primeiro login: "WASD para mover"
  - Primeiro inimigo: "Clique esquerdo para atacar"
  - Primeira morte: "Você pode ressuscitar no waypoint"
  - Primeiro item: "I para abrir o inventário"
- Sistema de flags: só mostra cada dica uma vez
- **Arquivo:** `voxygen/src/hud/tutorial.rs`

---

## FASE 4 — Segurança & Anti-Cheat (v0.2.9)
> Objetivo: endurecer o servidor contra trapaças de clientes conectados.

### 4.1 Anti-Cheat Server-Side Aprimorado
- **Speedhack detection:** rejeitar movimento se velocidade > máximo teórico
- **Damage validation:** rejeitar dano se valor excede limite da arma
- **Teleport detection:** rejeitar teleport não autorizado (diferença > threshold sem causa)
- **Rate limiting:** limitar comandos por segundo por jogador
- **Arquivo:** `server/src/sys/sentinel.rs` (expandir)

### 4.2 Validação de Integridade do Cliente
- Checksum dos binários críticos verificado na conexão
- Rejeitar cliente se hash não bater com versão esperada
- **Arquivos:** `server/src/login_provider.rs`, `common/net/src/msg/register.rs`

### 4.3 Sistema de Mute Persistente
- Mutes salvos em arquivo (sobrevivem restart do servidor)
- Expiração automática por tempo
- **Arquivos:** `server/src/automod.rs`, `server/src/lib.rs`

---

## FASE 5 — Polimento & Escala (v0.3.0)
> Objetivo: elevar o nível técnico e visual para um produto coeso.

### 5.1 IA de Mobs Aprimorada
- Mobs fogem quando HP < 20%
- Mobs chamam reforços (dentro de raio)
- Mobs usam habilidades especiais em intervalos
- **Arquivo:** `server/src/agent/behavior_tree/mod.rs`

### 5.2 Auto-Reconnect do Cliente
- Retry automático com backoff exponencial
- Tela de "Reconectando..." com spinner
- Mantém personagem logado por 30s para reconexão rápida
- **Arquivos:** `client/src/lib.rs`, `voxygen/src/menu/main/` (nova tela)

### 5.3 Lista de Servidores na Tela de Login
- Mostrar servidores disponíveis com ping, jogadores online, versão
- Permitir favoritar servidores
- **Arquivos:** `voxygen/src/menu/main/ui/`, `client/src/lib.rs`

### 5.4 Party Finder
- Tab "Procurando Grupo" no painel social
- Toggle "Disponível para grupo"
- Mostra jogadores disponíveis com nível e classe
- **Arquivos:** `voxygen/src/hud/friends_panel.rs`, `common/net/src/msg/` (novas variantes)

---

## Timeline Estimado

| Fase | Versão | Melhorias | Estimativa |
|------|--------|-----------|------------|
| 1 | v0.2.6 | Painel Admin + Comandos + Logs + Chat Grupo | 3-4 sessões |
| 2 | v0.2.7 | HP Grupo + Minimapa + Loot Raro + Block | 2-3 sessões |
| 3 | v0.2.8 | Lojas + PvP + XP Visual + Tutorial | 3-4 sessões |
| 4 | v0.2.9 | Anti-Cheat + Integridade + Mute Persistente | 2-3 sessões |
| 5 | v0.3.0 | IA + Auto-Reconnect + Lista Servidores + Party Finder | 4-5 sessões |

---

## Ordem de Implementação Recomendada

```
1.1 Painel Admin → 1.2 Comandos Admin → 1.3 Logs → 1.4 Chat Grupo
                                                        ↓
2.1 HP Grupo → 2.2 Minimapa Amigos → 2.3 Loot Raro → 2.4 Block
                                                        ↓
3.1 Lojas NPC → 3.2 PvP → 3.3 XP Visual → 3.4 Tutorial
                                                        ↓
4.1 Anti-Cheat → 4.2 Integridade → 4.3 Mute Persistente
                                                        ↓
5.1 IA Mobs → 5.2 Auto-Reconnect → 5.3 Lista Servidores → 5.4 Party Finder
```

Cada fase é independente o suficiente para ser releaseada sozinha.
As dependências mais fortes são: 1.2 depende de 1.1 (painel usa as ações admin).
Nenhum comando admin é acessível via chat — todos passam pelo painel visual (Ctrl+Alt+F12).

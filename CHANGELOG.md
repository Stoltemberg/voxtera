# Changelog — Voxtera

Todas as atualizações notáveis deste projeto são documentadas aqui.  
Formato baseado em [Keep a Changelog](https://keepachangelog.com/pt-BR/) e versionamento semântico.

---

## [Launcher 0.4.2] — 2026-07-31

### Corrigido
- **HTTPS no launcher congelado:** o `.app` agora carrega o bundle de autoridades certificadoras do `certifi` e cria um contexto TLS com validação de certificado e hostname obrigatória para API, manifesto e downloads. Não depende mais de `SSL_CERT_FILE` herdado do terminal.
- O ambiente de build PyInstaller foi declarado com versões fixadas em `launcher/requirements-build.txt`, evitando artefatos gerados com dependências implícitas do ambiente local.
- Erros de validação TLS exibem uma mensagem curta e acionável na interface; o traceback completo permanece em `launcher.log`.
- Em macOS, os botões usam texto escuro sobre a superfície nativa clara do Aqua para preservar contraste.

## [Launcher 0.4.1] — 2026-07-31

### Corrigido
- **macOS:** configurações e logs não são mais gravados dentro de `VoxteraLauncher.app`; agora usam `~/Library/Application Support/Voxtera`, compatível com App Translocation.
- **Verificação de atualização:** workers não chamam mais APIs do Tk. Uma fila é drenada somente pelo `mainloop`, eliminando o estado inicial permanente quando a atualização conclui em background; progresso de download é coalescido e drenado em lotes limitados.
- Releases RC/pré-release são ignorados no canal estável; releases exclusivos do launcher não escondem o ZIP do jogo compatível.
- Downloads exigem manifesto com SHA-256 válido para o arquivo/plataforma corretos e rejeitam ZIPs com caminhos ou taxas de expansão inseguros.

### Adicionado
- Área explícita **Configurações de instalação** com seleção persistente da pasta do jogo; o bundle do launcher não pode ser escolhido como destino.
- Log de diagnóstico em `launcher.log` no diretório de dados do usuário.
- Site usa URLs versionadas para os downloads e redirects da Vercel para assets de release, evitando artefatos estáticos desatualizados.

### Distribuição
- O build macOS desta versão é **x86_64 (Intel)**. Developer ID e notarização continuam pendentes antes de distribuição pública confiável.

## [0.4.0] — 2026-07-30

### Adicionado
- **Distribuição macOS universal:** o CI gera `Voxtera.app` com binário `x86_64` + `arm64` (via `lipo`), assets empacotados em `Contents/Resources/assets` e `VELOREN_ASSETS` definido pelo wrapper de lançamento.
- **Launcher Python multi-plataforma:** detecta Windows ou macOS, escolhe o artefato correto no release e prepara o ambiente com `VELOREN_ASSETS` antes de iniciar o jogo. No macOS gera um bundle `.app` `universal2`; o PyInstaller spec não usa mais extensões nativas (como Pillow) para manter compatibilidade universal.
- **Site com download por plataforma:** botões separados para `VoxteraLauncher.exe` (Windows) e `VoxteraLauncher.app.zip` (macOS).

### Polido
- **Aba Social (FriendsPanel, tecla `O`):**
  - Seção "Convidar Amigos" na aba `Group` só aparece quando há membros no grupo ou convite aberto pendente — não flutua mais sobre a lista vazia.
  - Botões `Kick` e `Promote` na aba `Group` com fonte `10pt` (antes `9pt`) e área de toque expandida (`56×22` e `64×22`).
  - Destaque visual da aba ativa: underline dourada (`rgba(1, 0.85, 0.3, 0.85)`) sob o botão da tab selecionada.
  - Ao receber um convite de grupo, o prompt "`{name}` convidou você para o grupo" é exibido ao lado dos botões **Aceitar/Rejeitar**.
  - Status dot verde de membros do grupo preserva legibilidade com cor extraída em variável.
- **i18n:** Novas entradas `hud-friends-group-invite-open` em EN e PT-BR.

### Corrigido
- **Launcher "Verificando atualizações" eterno (macOS):** `self.after(0, ...)` em thread daemon falhava com `RuntimeError: main thread is not in main loop` dentro do bundle `.app` PyInstaller, sem mudar a label — agora as atualizações de UI passam por uma `queue.Queue` thread-safe drenada pelo mainloop via `_pump_queue`. Garante que o status evolui mesmo quando Tk descarta a chamada direta. Coberto por testes em `TkThreadSafeAfterTests`.

### Alterado
- G. 825 8d8feqq549084a30 daable323d9882f6aa2139ecf3b3ecf substituído por `path = toString ./.` para compatibilidade com Nix 2.35 puro (em vez de `./.`).
- `.gitignore` inclui exceção para `!site/public/downloads/VoxteraLauncher.app.zip`.

## [0.2.6] — 2026-07-29

### Adicionado
- **Party Chat (Fase 1.4 do roadmap multiplayer):** novo canal de chat de grupo acessível pelos comandos `/p <mensagem>`, `/party <mensagem>`, `/g <mensagem>` e `/group <mensagem>`.
- **Aba "Grupo" no chat HUD** (4ª aba pré-configurada) — filtra apenas mensagens do grupo (ChatType::Group + GroupMeta + mortes/activity de membros do grupo). Perfis já existentes recebem a aba por uma migração única, sem recriá-la se o jogador a remover depois.
- **Aliases de comando `/p` e `/party`** registrados via novo método `ServerChatCommand::aliases()` no `common/src/cmd.rs` — os aliases são resolvidos pelo `FromStr` junto com `keyword()` e `short_keyword()`.
- **Switch automático de ChatMode:** ao digitar `/p` (sem mensagem) o cliente troca o modo de chat para Grupo, igual ao comportamento de `/g` e `/group`.
- **i18n PT-BR + EN atualizado:** `hud-chat-meta-group-join-hint` agora menciona `/p` além de `/g` e `/group`.

### Alterado
- `handle_party_chat` (server/src/lib.rs) não rejeita mais mensagem vazia com erro de "Uso"; em vez disso, alterna o `ChatMode` do jogador para `Group` e notifica o cliente, espelhando o handler upstream `handle_group`.

---

## [0.2.4] — 2026-07-22

### Adicionado
- Painel social com lista de amigos visual completa.
- Botão **"Convidar para grupo"** no painel de amigos.
- **Contador de jogadores online** no canto superior direito da tela (HUD).
- **Spawn protection**: 30 segundos de invulnerabilidade ao nascer/respawnar. A proteção acaba automaticamente ou se o jogador atacar primeiro.
- **Comando `/announce`** para uso administrativo — broadcast global para todos os jogadores conectados.
- **Persistência da lista de amigos** — a lista agora é salva em `friends.ron` no servidor e não desaparece mais ao fechar o jogo.
- Tradução PT-BR dos novos textos e fluxos do sistema social.

### Corrigido
- Ajustes no fluxo de aceitação/recusa/remoção de amigos no painel visual.
- Compatibilidade PT-BR/EN nos arquivos de idioma do HUD social.

---

## [0.2.3] — 2026-07-21

### Adicionado
- Plano de implementação do **novo launcher Voxtera** em Tauri 2 (React + TypeScript + Rust). Inclui tarefas detalhadas para download com retomada, validação SHA-256, staging/rollback e empacotamento Windows.
- Planejamento do **site oficial Voxtera** para deploy na Vercel, com identidade visual fantasy-voxel coerente com o jogo e fluxo direto de download do launcher.

### Documentação
- `docs/superpowers/plans/2026-07-21-tauri-launcher.md`
- `docs/superpowers/plans/2026-07-21-voxtera-website.md`

---

## [0.2.2] — 2026-07-21

### Adicionado
- **Design specs** do launcher e do site oficial. Define a arquitetura completa do novo launcher (substituição do Python/Tkinter atual por Tauri 2) e o design visual do site na Vercel.
- Escopo fechado para a primeira release do launcher: instalar, atualizar, reparar, escolher pasta, lançar o jogo, retomar downloads interrompidos, validar integridade do arquivo e rollback automático em caso de falha.
- Restrições explícitas da primeira versão: sem auto-update do launcher, sem delta patches, sem contas de usuário, apenas Windows 10/11 x64, canal único Preview (GitHub pre-release).

### Documentação
- `docs/superpowers/specs/2026-07-21-launcher-site-design.md`

---

**Legenda:** `[0.2.4]`, `[0.2.3]`, `[0.2.2]` são versões conceituais do repositório. O jogo continua em desenvolvimento ativo.

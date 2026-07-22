# Changelog — Voxtera

Todas as atualizações notáveis deste projeto são documentadas aqui.  
Formato baseado em [Keep a Changelog](https://keepachangelog.com/pt-BR/) e versionamento semântico.

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

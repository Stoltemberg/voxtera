# Voxtera — Plano de Melhorias e Novas Implementações

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Identificar e planejar melhorias para o projeto Voxtera (fork do Veloren), cobrindo correções técnicas, novas features, melhorias visuais e integração Supabase.

**Architecture:** Jogo voxel 3D em Rust (specs ECS + wgpu + iced UI). Cliente (`voxygen`) conecta ao servidor (`server-cli`) via TCP. Auth via Supabase (ES256 JWT). Build: `nightly-2026-06-13-x86_64-pc-windows-msvc`.

**Tech Stack:** Rust (nightly), specs ECS, wgpu, iced, Supabase (Auth + PostgreSQL), ureq, jsonwebtoken

---

## Estado Atual (Resumo)

### ✅ Implementado
- Rebrand Veloren → Voxtera (título, menus, textos, 33 arquivos modificados)
- HitFlash ECS system + crosshair flash + impact particles
- Melhorias visuais (saturação +15%, glow +33%, bloom intensificado, gamma/exposure/ambiance)
- Tela de registro in-game (3 colunas, centralizada, estilo consistente)
- Supabase auth no cliente (`supabase_auth.rs` — sign_up, sign_in via ureq)
- Supabase JWT ES256 no servidor (`login_provider.rs` — validação local)
- Schema SQL (profiles, characters, user_settings, RLS, triggers)
- Binário renomeado `Voxtera.exe`
- Validação de username removida no login (aceita emails)

### 🔴 Problemas Conhecidos
- **Auth flow incompleto:** `sign_in` retorna 400 — Supabase exige confirmação de email (desabilitar no dashboard)
- **RegisterSubmit não conectado:** Handler chama `SupabaseAuthClient` mas não trata resposta nem navega para tela de login
- **Username no login vs Supabase:** Login envia username mas Supabase precisa de email — campo precisa ser renomeado
- **`v_logo` não lido:** Warning de campo não utilizado em `menu/main/ui/mod.rs`
- **`info_message` e `localization` não usados:** Após remoção da validação de username
- **Servidor não salva personagens no Supabase:** Schema SQL criado mas servidor não usa tabelas `characters`/`profiles`
- **SupabaseAuthClient no cliente:** Usa `ureq` (sync) dentro de contexto async — pode causar problemas de performance

---

## Fase 1: Correções Críticas (Auth Flow)

### Task 1: Desabilitar Confirmação de Email no Supabase
**Objective:** Permitir login imediato após registro

**Action:** Manual — acessar https://supabase.com/dashboard/project/gcfavlnisyhdwseuvzpd/auth/providers → Email → desmarcar "Confirm email"

**Verification:** Criar conta no jogo → fazer login imediatamente → sem erro 400

---

### Task 2: Corrigir Handler RegisterSubmit
**Objective:** Após registro, mostrar sucesso e navegar para tela de login

**Files:**
- Modify: `voxygen/src/menu/main/ui/mod.rs:627-670`

**Current code (broken):**
```rust
Message::RegisterSubmit => {
    if self.showing == Showing::Register {
        let auth_client = client::supabase_auth::SupabaseAuthClient::voxtera();
        match auth_client.sign_up(&email, &password, &username) {
            Ok(response) => {
                if response.access_token.is_some() {
                    // TODO: save token, navigate to login
                }
            }
            Err(e) => {
                // TODO: show error
            }
        }
    }
}
```

**Fix:** Adicionar feedback visual (mensagem de sucesso/erro) e navegação para `Showing::Login`

**Step 1:** Adicionar campo `register_message: Option<String>` ao struct Controls

**Step 2:** No handler RegisterSubmit:
- Se `Ok` → `self.register_message = Some("Conta criada com sucesso!".into())` + `self.showing = Showing::Login`
- Se `Err` → `self.register_message = Some(format!("Erro: {}", e))`

**Step 3:** Na tela de registro, mostrar `register_message` se existir

**Verification:** Compilar → criar conta → ver mensagem de sucesso → tela de login

---

### Task 3: Renomear Campo "Username" para "Email" na Tela de Login
**Objective:** Deixar claro que o campo aceita email (não username)

**Files:**
- Modify: `voxygen/src/menu/main/ui/login.rs` — placeholder do campo username
- Modify: `assets/voxygen/i18n/en/main.ftl` — string de tradução
- Modify: `assets/voxygen/i18n/pt-BR/main.ftl` — string de tradução

**Changes:**
```ftl
# en/main.ftl
main-username = Email

# pt-BR/main.ftl
main-username = Email
```

**Verification:** Compilar → tela de login mostra "Email" em vez de "Nome de Usuário"

---

### Task 4: Limpar Warnings de Compilação
**Objective:** Remover warnings de variáveis não utilizadas

**Files:**
- Modify: `voxygen/src/menu/main/mod.rs:720` — prefixar `info_message` com `_`
- Modify: `voxygen/src/menu/main/mod.rs:731` — prefixar `localization` com `_`
- Modify: `voxygen/src/menu/main/ui/mod.rs:46` — remover campo `v_logo` ou usar `_v_logo`

**Verification:** `cargo check --workspace` sem warnings

---

## Fase 2: Integração Supabase Completa

### Task 5: Converter SupabaseAuthClient para Async
**Objective:** Usar `reqwest` (async) em vez de `ureq` (sync) para não bloquear o runtime

**Files:**
- Modify: `client/src/supabase_auth.rs` — reescrever com `reqwest::Client`
- Modify: `client/Cargo.toml` — adicionar `reqwest` se não presente

**Approach:**
```rust
pub async fn sign_in(&self, email: &str, password: &str) -> Result<AuthResponse, SupabaseAuthError> {
    let client = reqwest::Client::new();
    let response = client.post(&format!("{}/auth/v1/token?grant_type=password", self.config.project_url))
        .header("apikey", &self.config.anon_key)
        .json(&serde_json::json!({"email": email, "password": password}))
        .send()
        .await?;
    // ...
}
```

**Update call site in `client/src/lib.rs:1151-1165`:**
```rust
if addr.contains("supabase.co") {
    let supabase_client = crate::supabase_auth::SupabaseAuthClient::voxtera();
    match supabase_client.sign_in(username, password).await {
        // ...
    }
}
```

**Verification:** `cargo check --workspace` → compila sem erros

---

### Task 6: Salvar Token Supabase Após Login
**Objective:** Persistir o access_token para reconexão automática

**Files:**
- Modify: `voxygen/src/menu/main/ui/mod.rs` — armazenar token no state
- Modify: `voxygen/src/settings/mod.rs` — campo `supabase_token: Option<String>`

**Approach:**
1. Após `sign_in` bem-sucedido, salvar `access_token` no state do menu
2. Ao conectar ao servidor, enviar token como `token_or_username`
3. O servidor já valida o token via ES256

**Verification:** Login → fechar jogo → reabrir → reconectar automaticamente

---

### Task 7: Integrar Tabelas Supabase no Servidor
**Objective:** Usar `profiles` e `characters` do Supabase para persistir dados

**Files:**
- Create: `server/src/supabase_db.rs` — módulo de acesso ao Supabase DB
- Modify: `server/src/login_provider.rs` — após validar JWT, buscar/criar profile
- Modify: `server/Cargo.toml` — adicionar `postgrest` ou usar `reqwest` direto

**Approach:**
1. Após validar JWT, extrair UUID do token
2. Buscar profile em `profiles` via REST API do Supabase
3. Se não existir, criar com username do `user_metadata`
4. Buscar personagens em `characters` por `user_id`

**API:**
```
GET https://gcfavlnisyhdwseuvzpd.supabase.co/rest/v1/profiles?id=eq.{uuid}
Authorization: Bearer {service_role_key}
apikey: {service_role_key}
```

**Verification:** Login → servidor busca profile → personagens carregados

---

## Fase 3: Melhorias de UX

### Task 8: Tela de Loading/Conexão
**Objective:** Mostrar progresso ao conectar ao servidor

**Files:**
- Modify: `voxygen/src/menu/main/ui/mod.rs` — estado de loading
- Modify: `voxygen/src/menu/main/ui/login.rs` — indicador visual

**Approach:**
1. Quando "Multijogador" é clicado, mostrar "Conectando..." com spinner
2. Se auth falhar, mostrar erro na tela (não popup)
3. Se sucesso, transição suave para o jogo

**Verification:** Clicar "Multijogador" → ver "Conectando..." → entrar no jogo

---

### Task 9: Crosshair Hit Indicator Melhorado
**Objective:** Indicar direção do dano recebido com indicadores visuais

**Files:**
- Modify: `voxygen/src/hud/mod.rs` — adicionar hit direction indicators
- Create: `voxygen/src/hud/hit_indicator.rs` — novo widget

**Approach:**
1. Quando jogador recebe dano, calcular ângulo de origem
2. Mostrar indicador vermelho na borda da tela na direção do hit
3. Fade out em ~500ms

**Verification:** Levar dano → ver indicador vermelho na borda → desaparece

---

### Task 10: Minimap Improvements
**Objective:** Adicionar marcadores de jogadores e pontos de interesse no minimap

**Files:**
- Modify: `voxygen/src/hud/minimap.rs` — adicionar marcadores

**Approach:**
1. Mostrar posições de outros jogadores como pontos azuis
2. Mostrar waypoints como pontos amarelos
3. Adicionar legenda

**Verification:** Abrir minimap → ver jogadores e waypoints marcados

---

## Fase 4: Features Novas

### Task 11: Sistema de Amigos
**Objective:** Permitir adicionar e ver amigos online

**Files:**
- Create: `server/src/friends.rs` — lógica de amizade
- Modify: `supabase_schema.sql` — tabela `friendships`
- Modify: `voxygen/src/hud/social.rs` — UI de amigos

**Schema:**
```sql
CREATE TABLE public.friendships (
    user_id UUID REFERENCES public.profiles(id),
    friend_id UUID REFERENCES public.profiles(id),
    status TEXT DEFAULT 'pending',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (user_id, friend_id)
);
```

**Verification:** Adicionar amigo → ver status online → convite para grupo

---

### Task 12: Chat Global Aprimorado
**Objective:** Adicionar canais de chat (global, local, grupo, PM)

**Files:**
- Modify: `voxygen/src/hud/chat.rs` — abas de chat
- Modify: `server/src/chat.rs` — roteamento de mensagens

**Approach:**
1. Adicionar abas: Global, Local, Grupo, PM
2. Filtro por canal
3. Histórico persistente (últimas 100 mensagens)

**Verification:** Digitar no chat → ver mensagem no canal correto → histórico persiste

---

### Task 13: Tutorial Interativo
**Objective:** Guiar novos jogadores pelos controles básicos

**Files:**
- Create: `voxygen/src/hud/tutorial.rs` — sistema de tutorial
- Modify: `voxygen/src/session/mod.rs` — detecção de ações

**Steps do Tutorial:**
1. "Use WASD para se mover"
2. "Pressione Espaço para pular"
3. "Clique esquerdo para atacar"
4. "Pressione E para interagir"
5. "Abra o inventário com I"

**Verification:** Novo jogador → ver tutorial → completar etapas → tutorial desaparece

---

### Task 14: Sistema de Loot Melhorado
**Objective:** Melhorar feedback visual de loot drop

**Files:**
- Modify: `voxygen/src/scene/figure/mod.rs` — glow em itens no chão
- Modify: `voxygen/src/hud/loot.rs` — tooltip ao passar mouse

**Approach:**
1. Itens no chão brilham com cor baseada na raridade
2. Tooltip mostra nome + stats ao passar mouse
3. Itens raros têm partículas especiais

**Verification:** Matar inimigo → loot drop → ver brilho → hover mostra stats

---

## Fase 5: Performance e Estabilidade

### Task 15: Otimização de Render
**Objective:** Melhorar FPS em cenas densas

**Files:**
- Modify: `voxygen/src/render/mod.rs` — frustum culling
- Modify: `voxygen/src/scene/terrain.rs` — LOD dinâmico

**Approach:**
1. Frustum culling mais agressivo
2. LOD dinâmico baseado em FPS
3. Reduzir draw calls com batching

**Verification:** FPS estável em cenas densas (>30fps em hardware mínimo)

---

### Task 16: Tratamento de Erros de Rede
**Objective:** Reconexão automática e feedback de status

**Files:**
- Modify: `voxygen/src/menu/main/mod.rs` — retry logic
- Modify: `client/src/lib.rs` — reconnect on disconnect

**Approach:**
1. Se conexão cair, tentar reconectar 3x
2. Mostrar status "Reconectando..." na tela
3. Se falhar, voltar ao menu com mensagem de erro

**Verification:** Desconectar internet → ver "Reconectando..." → reconectar → continuar jogando

---

## Ordem de Implementação Recomendada

1. **Fase 1** (Correções Críticas) — Tasks 1-4 — ~30 min
2. **Fase 2** (Supabase Completo) — Tasks 5-7 — ~2h
3. **Fase 3** (UX) — Tasks 8-10 — ~3h
4. **Fase 4** (Features) — Tasks 11-14 — ~8h
5. **Fase 5** (Performance) — Tasks 15-16 — ~4h

---

## Riscos e Tradeoffs

| Risco | Impacto | Mitigação |
|-------|---------|-----------|
| Supabase rate limits | Alto | Implementar cache de tokens |
| JWT secret rotation | Médio | Buscar JWKS discovery URL |
| Performance do ureq sync | Alto | Migrar para reqwest async |
| Schema SQL não aplicado | Alto | Verificar no Supabase Dashboard |
| Breaking changes no Veloren upstream | Médio | Manter fork separado |

---

## Perguntas Abertas

1. **Service Role Key:** Deve ser usada no servidor para acessar tabelas? Ou usar anon key com RLS?
2. **Persistência de personagens:** Usar Supabase ou SQLite local (como o Veloren original)?
3. **Multi-servidor:** Suportar múltiplos servidores ou apenas um fixo?
4. **Anti-cheat:** Implementar validação server-side de movimentos/dano?
5. **Updates:** Como distribuir atualizações para amigos? (auto-update, manual, launcher?)

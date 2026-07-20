# Plano de Implementação — Melhorias de Jogabilidade (Opções 1 e 2)

## 1. Dicas Contextuais Durante o Combate

### Objetivo
Exibir tooltip contextual quando o jogador mira em um inimigo, mostrando:
- Tipo de arma equipada vs. resistência/fraca do inimigo
- Dano esperado considerando armor/protection
- Bônus/penalidades por tipo de ataque (melee, ranged, beam)

### Arquivos a modificar

#### `common/src/comp/inventory/item/armor.rs`
- Adicionar método `fn resistance_summary(&self) -> String` que retorna tipo de resistência (physical, fire, ice, etc.)
- Adicionar método `fn weakness_summary(&self) -> String` para fraquezas

#### `common/src/comp/inventory/item/tool.rs`  
- Adicionar método `fn damage_type(&self) -> DamageType` (slash, pierce, blunt, fire, ice, etc.)
- Adicionar método `fn damage_vs_armor(&self, armor: &Protection) -> f32` para calcular efetividade

#### `common/src/combat.rs`
- Adicionar struct `CombatTip` com campos: weapon_type, armor_type, effectiveness_rating, suggested_action
- Adicionar função `fn generate_combat_tip(attacker: &AttackerInfo, target: &TargetInfo) -> Option<CombatTip>`

#### `voxygen/src/hud/skillbar.rs` (ou novo widget `combat_tip.rs`)
- Novo widget `CombatTipWidget` que aparece quando:
  - Player está em modo combate (weapon drawn)
  - Cursor/target está sobre entidade inimiga
- Mostra:
  - Ícone do tipo de dano
  - Barra de efetividade (verde/amarelo/vermelho)
  - Texto curto: "Efetivo contra armadura leve" ou "Inefetivo contra escudo"

#### `voxygen/src/session/mod.rs`
- Adicionar sistema que detecta entidade sob mira e gera CombatTip
- Passar CombatTip para HUD

### Fluxo
1. Player equipa arma → `tool.rs` retorna tipo de dano
2. Sistema de targeting identifica inimigo → `armor.rs` retorna proteção
3. `combat.rs` calcula efetividade → gera `CombatTip`
4. HUD mostra tooltip contextual próximo ao crosshair

---

## 2. Feedback de Hit Mais Visível

### Objetivo
Tornar acertos mais satisfatórios com:
- Flash branco/vermelho no modelo do inimigo ao ser atingido
- Shake/flash no crosshair
- Número de dano flutuante maior e mais visível
- Som de hit mais impactante (já existe, mas pode ser reforçado)

### Arquivos a modificar

#### `common/src/comp/visual.rs`
- Adicionar componente `HitFlash` com campos:
  ```rust
  pub struct HitFlash {
      pub timer: f32,        // duração do flash
      pub color: Rgb<f32>,   // cor do flash (branco para hit normal, vermelho para crítico)
      pub intensity: f32,    // 0.0 a 1.0
  }
  ```
- Registrar como componente ECS

#### `common/src/event.rs`
- Já existe `HealthChangeEvent` — adicionar campo `is_critical: bool`

#### `voxygen/src/scene/figure/mod.rs`
- No sistema de renderização de figuras, detectar `HitFlash` e aplicar:
  - Override de cor temporário no shader
  - Emissão de luz branca/vermelha por ~100ms
  - Escala ligeiramente maior (1.05x) por ~50ms

#### `voxygen/src/scene/particle.rs`
- Adicionar emissores de partículas no ponto de impacto:
  - Partículas de sangue (vermelho)
  - Partículas de faísca (se atingir armadura metálica)
  - Partículas de energia (se atingir com dano elemental)

#### `voxygen/src/hud/skillbar.rs` (já existe)
- Crosshair flash: quando `HealthChangeEvent` é recebido para inimigo na mira:
  - Flash branco no crosshair por ~50ms
  - Leve escala (1.1x) por ~80ms
  - Opcional: screen shake sutil

#### `voxygen/src/ecs/sys/animation.rs` (ou similar)
- Detectar `HealthChangeEvent` e aplicar animação de "recoil" no inimigo:
  - Knockback visual para trás
  - Animação de stagger se dano > threshold

### Fluxo
1. Ataque acerta → `HealthChangeEvent` é emitido com `is_critical`
2. Sistema de visual detecta → adiciona `HitFlash` ao componente
3. Render loop aplica flash de cor no modelo
4. Particle system emite partículas de impacto
5. HUD aplica flash no crosshair
6. Animation system aplica recoil/knockback

---

## Ordem de Implementação Sugerida

1. **Feedback de Hit (Feature 2)** — mais visível, mais impacto imediato
   - Começar por `HitFlash` component + render
   - Depois partículas
   - Depois crosshair flash

2. **Dicas Contextuais (Feature 1)** — mais complexa, precisa de mais testes
   - Começar por funções de cálculo de efetividade
   - Depois tooltip widget
   - Depois integração com targeting

## Notas Técnicas
- O Veloren usa `conrod` para UI, então widgets seguem o padrão `WidgetCommon`
- ECS é `specs`, então novos componentes precisam ser registrados
- Render usa `wgpu`, então shaders podem precisar de modificação para flash effects
- Sistema de partículas já existe em `voxygen/src/scene/particle.rs`

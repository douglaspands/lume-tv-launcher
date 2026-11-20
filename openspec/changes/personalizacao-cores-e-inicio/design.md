# Design: personalização de cores e estilos da tela inicial

## Context

O motivo e o escopo estão em [proposal.md](proposal.md). A arquitetura atual do Lume molda diretamente esta solução:

- **Tema estático (`crates/lume-core/src/theme.rs`, `build.rs`, `themes/lume/theme.css`)**: hoje `build.rs` compila `theme.css` em `const THEME: Theme` e `pub mod vars` (`PRIMARY`, `ON_PRIMARY`, `ACCENT`, `ON_ACCENT`), sem suporte a override em tempo de execução. `Theme` é uma struct `Copy` de poucos kilobytes (sem heap). `Painter::style` e `Painter::focus_style` resolvem estilos direto do `THEME` global, e cerca de 10 módulos de UI (`topbar.rs`, `apps.rs`, `page.rs`, `splash.rs`, `settings.rs`, `tools/common.rs`, `editor.rs`, `keymap.rs`, `receive.rs`, além de `companion/page.rs`) leem `vars::PRIMARY` / `vars::ON_PRIMARY` diretamente.
- **Persistência (`crates/lume-core/src/home/config.rs`, `storage.rs`)**: `HomeConfig` é salvo em `files/config.v1` no formato `storage::Kv` (`lume=1` ... `#end`). Chaves ausentes recebem valores padrão e chaves desconhecidas ou fora de faixa são ignoradas/reparadas. A persistência ocorre imediatamente via `Effect::SaveConfig` tratado por `Shell::finish`.
- **Navegação e foco (`crates/lume-core/src/app/{mod,topbar,home,apps,dock,editor,settings,context_menu,splash}.rs`, `ui/focus.rs`)**:
  - As abas usam `Screen::Main(Tab::Home | Tab::Apps)` com `topbar::row` (`TAB_HOME`, `TAB_APPS`, `INPUTS?`, `GEAR`) e troca imediata no `LumeApp::horizontal` com `Anim::TabFade` (180 ms).
  - Os IDs de foco do Dock (`FAVORITE_BASE = 1000..`) e da grade (`GRID_BASE = 2000..`, `GRID_SETTINGS = 2999`) já são disjuntos, assim como `ContextMenu::Origin` (`Origin::Dock` vs `Origin::Grid`).
  - `HomeScreen` desenha o papel de parede vivo (`dimmed = false`); `AppsScreen` desenha o papel de parede escurecido (`dimmed = true`) e a grade de 6 colunas em **3 passes** (boxes -> images -> text, mantendo no máximo 1 batch de texto no clip) com culling vertical.
  - `Shell::is_busy` bloqueia o carregamento preguiçoso de ícones enquanto `dirty || app.is_moving(now_ms)`. `splash_images` já aguarda os ícones do Dock e os tiles visíveis da grade.
- **Restrições fixas**: construção **100% Rust** (zero Java/XML novo), orçamento de memória privada suja `<= 32 MB` em primeiro plano e `<= 10 MB` em segundo, no máximo `1%` de quadros `>= 33 ms` (TCL 55C715 1080p e AIWA 720p `low_ram`), e **nunca** desenhar camada translúcida de tela cheia sobre o papel de parede.

## Goals / Non-Goals

**Goals:**
- Permitir troca dinâmica da cor de destaque (`--primary`) entre 10 presets curados com pré-visualização instantânea no D-pad, zero alocação extra de VRAM/textura e contraste legível a 3 metros (incluindo `Branco` em cartão claro e `Preto` em fundos escuros).
- **Fazer o logo do Lume (com "L" e ponto transparentes/vazados) e a tela de carregamento (Splash: logo + fundo escuro T7) acompanharem a cor escolhida desde o 1º quadro, sem nunca exibir o azul padrão antes.**
- Suportar os 5 estilos de tela inicial (`Abas à esquerda`, `Abas centralizadas`, `Descer`, `Espiada`, `No Dock`) mantendo 60 fps, restauração de `ReturnPoint`, comportamento previsível de `Home`/`Back` e `back_from_settings()`, em 720p e 1080p.
- Manter 100% de retrocompatibilidade em `files/config.v1` sem quebrar instalações existentes nem alterar Java.

- **Non-Goals:**
  - Seletor arbitrário de cor (RGB/HEX livre) ou temas externos carregados de disco.
  - Mudar código Java para a *starting window* (apenas neutralizar a cor estática `#0C171F` → `#0A0D10` no recurso XML `styles.xml`, zero Java).
  - Mudar os orçamentos de memória/quadros da spec `resource-efficiency` ou a régua de 4 unidades do `design-system`.

## Decisions

### 1. Override dinâmico de cor de destaque em runtime (zero heap, zero VRAM extra)

Todo elemento que usa `--primary` no Lume é geometria/cor de vértice (`DisplayList`: preenchimentos, anéis, logo vetorial e cor de texto). Nenhuma textura de GPU é pré-pintada com `--primary`.

- **Paleta curada (`AccentColor` / `AccentPalette`)**: definida em `crates/lume-core/src/theme.rs` como tipos `Copy` estáticos com os 10 presets:

| ID (`config.v1`) | Nome (PT-BR / EN) | `primary` | `on_primary` | `splash_bg` (T7) | Regras especiais |
|---|---|---|---|---|---|
| `blue` *(padrão)* | Azul Lume / Lume Blue | `#4DB3FF` | `#0C171F` | `#0C171F` | — |
| `aurora` | Aurora / Aurora | `#00C0B2` | `#0C171F` | `#071B1A` | — |
| `emerald` | Esmeralda / Emerald | `#57C071` | `#0C171F` | `#091A10` | — |
| `amber` | Âmbar / Amber | `#EC9A00` | `#0C171F` | `#1C1406` | — |
| `coral` | Coral / Coral | `#FF8163` | `#0C171F` | `#1E100C` | — |
| `rose` | Rosa / Rose | `#FA7ABC` | `#0C171F` | `#1D0E17` | — |
| `violet` | Violeta / Violet | `#AC96FF` | `#0C171F` | `#130F21` | — |
| `red` | Vermelho / Red | `#FF5449` | `#0C171F` | `#1F0B0A` | — |
| `white` | Branco / White | `#F1F4F9` | `#0C171F` | `#141619` | Foco em cartão claro (`menu_focus_bg: #1D2730`, `menu_focus_fg: #F1F4F9`) |
| `black` | Preto / Black | `#11151A` | `#F1F4F9` | `#07090C` | `contrast_ring: Some(rgba(255,255,255,0.45))` em fundos escuros e halo claro sutil no Splash |

- **Derivação de `Theme` em memória (`Theme::with_accent`)**:
  - `LumeApp` mantém `active_theme: Theme` (derivado em O(1) de `const THEME` sem alocações) e `preview_accent: Option<AccentColor>` enquanto o foco está na linha de swatches do editor.
  - `Theme::with_accent(accent: AccentColor)` copia `THEME` e substitui apenas as propriedades dos componentes que dependem de `--primary`/`--on-primary` (`.icon-button:focus`, `.button:focus`, `.button-secondary:focus`, `.button-danger:focus`, `.tile:focus`, `.settings-item:focus`, `.editor-row:focus`, `.chip:active`, `.chip:focus`) e, nos componentes do cartão claro (`.menu-item:focus`, `.menu-button:focus`, `.menu-button-danger:focus`), aplica `accent.menu_focus_bg()` / `accent.menu_focus_fg()` (que diverge de `primary`/`on_primary` apenas para `Branco`, evitando branco sobre `#F1F4F9`).
  - `Painter` passa a carregar `theme: &'a Theme` e `accent: AccentPalette`, expondo `p.style(...)`, `p.focus_style(...)`, `p.primary()`, `p.on_primary()`, `p.accent()`.
  - **Migração dos usos diretos de `vars::PRIMARY`/`vars::ON_PRIMARY`**: os ~10 arquivos de produção (`topbar.rs`, `apps.rs`, `page.rs`, `splash.rs`, `settings.rs`, `tools/common.rs`, `editor.rs`, `keymap.rs`, `receive.rs`) passam a ler de `p.primary()` / `p.on_primary()`. Em `companion/page.rs`, `render()` substitui `--primary` pelo hex ativo em runtime para manter coerência na página do celular.
  - **Borda externa de contraste (`Preto`)**: quando `accent.contrast_ring` é `Some(color)`, `Painter::draw_box` (quando o estilo resolvido possui anel/fill de foco na cor primária sobre fundo escuro) e os anéis manuais de foco do editor desenham 1 anel externo fino (`1.5 px`) em `rgba(255, 255, 255, 0.45)`. Como apenas 1 elemento tem foco por quadro, o custo de GPU é de 1 retângulo extra.
  - **Logo vazado (`L` e ponto transparentes) e Splash imediatamente na cor escolhida**:
    - `logo::draw_mark` passa a desenhar o quadrado arredondado na cor `p.primary()` com o **`L` e o ponto verdadeiramente transparentes (vazados)** por operação SDF no fragment shader de `lume-render` (1 único quad SDF subtraindo as 3 formas internas, zero textura/FBO extra e sem artefato de duplo blend durante o fade do Splash), permitindo que o papel de parede, o fundo da tela ou o fundo T7 do Splash apareçam diretamente através do `L` e do ponto. Na página do celular (`companion/page.rs`), o SVG inline aplica `<mask>` equivalente.
    - No Splash, `splash_bg_color` usa **`accent.splash_bg` diretamente desde o primeiro quadro** (sem interpolação a partir do azul `#0C171F`), tanto na abertura a frio quanto no retorno a quente ao pressionar `Home`, e o clear do framebuffer GL durante o Splash usa `accent.splash_bg`. Apenas durante `wallpaper_reveal()` o fundo mistura com `vars::BG` conforme o papel de parede entra (zero overdraw em `Phase::Normal`).
    - Em `android/app/src/main/res/values/styles.xml`, `lume_background` passa de `#0C171F` (azulado) para escuro neutro `#0A0D10`, evitando qualquer flash azul na janela inicial do Android antes do 1º quadro em Rust. Para `Preto`, um anel/halo sutil claro atrás do logo garante legibilidade sobre `#07090C`.

### 2. Modelo de dados e persistência retrocompatível (`files/config.v1`)

`HomeConfig` em `crates/lume-core/src/home/config.rs` evolui o formato dos favoritos para `DockIconShape` e ganha chaves opcionais na serialização `storage::Kv` (mantendo `VERSION = 1`):

```rust
pub enum DockIconShape {
    Round,  // padrão ("round"): circular
    Square, // ("square"): quadrado de cantos arredondados
    Banner, // ("banner"): retangular 16:9 (banner de TV)
}

pub struct HomeConfig {
    pub favorites: Vec<String>,
    pub dock_shape: DockIconShape,             // chave: dock_shape ("round" | "square" | "banner", padrão: round; fallback de round_favorites)
    pub wallpaper: WallpaperId,
    pub custom: Option<Applied>,
    pub accent_color: AccentColor,             // chave: accent_color (padrão: blue)
    pub tabs_visible: bool,                    // chave: tabs_visible ("1" | "0", padrão: true)
    pub tabs_align: TabsAlign,                 // chave: tabs_align ("left" | "center", padrão: left)
    pub tabless_mode: TablessMode,             // chave: tabless_mode ("slide" | "peek" | "dock", padrão: slide)
    pub all_apps_dock_index: Option<usize>,    // chave: all_apps_dock_index (0..=favorites.len(), padrão: None = fim)
}
```

- **Retrocompatibilidade do formato do Dock (`dock_shape` ↔ `round_favorites`)**:
  - Ao carregar `config.v1`: se `dock_shape` existir e for válido (`round`/`square`/`banner`), usa-o; se ausente, lê a chave legada `round_favorites` (`"0"` → `Square`, `"1"` ou ausente → `Round`). Valores desconhecidos caem em `Round`.
  - Ao salvar `config.v1`: grava `dock_shape` e também `round_favorites` (`"1"` para `Round`, `"0"` para `Square`/`Banner`) para manter rollback seguro em versões anteriores.
- **Formato `Retangular` (`Banner`, 16:9) no Dock (`dock.rs`, `home.rs`, `splash.rs`, `shell/mod.rs`)**:
  - **Arte sem custo extra de VRAM**: em `Round` e `Square`, o dock exibe o ícone quadrado do app (`icon_image`). Em `Banner`, o dock reutiliza exatamente o banner 16:9 já carregado no atlas de `AppIcons` para a grade de Apps (`banner_image`), com fallback para cor do app + inicial/ícone. `splash_images` e `dock_icons_wanted` passam a solicitar o banner quando `dock_shape == Banner`.
  - **Geometria 16:9 dentro da área segura**: cada item retangular parte de `112 × 64` (razão 16:9 na régua de 4 px, mesma altura de repouso `BASE = 64.0`) com cantos arredondados proporcionais (`--shape-md`). Quando a largura total com Magnification e `MOVE_ROOM` exceder `MAX_BAR_WIDTH` (`1152 px`, ex.: de 9 a 13 itens), `dock::layout_with_room` reduz proporcionalmente a base mantendo a razão 16:9 e o vão (`GAP`), garantindo que até 12 favoritos + `"Todos os apps"` (13 itens) permaneçam sempre dentro da área segura sem rolagem horizontal.
  - **Item `"Todos os apps"` no modo `No Dock`**: acompanha o formato ativo (`Round` circular, `Square` quadrado arredondado, `Banner` retangular 16:9) com fundo sólido liso na cor ativa `p.primary()` (sem gradiente; com borda sutil `contrast_ring` quando `Preto` estiver ativo) e **grade 3×3 de pontos circulares** em `p.on_primary()` desenhada diretamente via SDF (zero textura extra) — centralizada em `Round`/`Square` e posicionada à esquerda do rótulo traduzido `"Apps"` (`strings::tab_apps`, em `p.on_primary()`) no formato `Banner` 16:9, escalando proporcionalmente conforme o tamanho do item no dock.

- **Por que `all_apps_dock_index` fica separado de `favorites: Vec<String>`?**
  1. Preserva a invariante de até `MAX_FAVORITES = 12` apps reais em `favorites`: no modo `No Dock`, o botão **"Todos os apps"** é um **slot extra** além dos até 12 favoritos (total de até 13 ícones no Dock).
  2. Evita strings sentinela falsas de pacote Android dentro de `favorites`, impedindo interferência com `remove_uninstalled(&catalog)`, `UsageHistory` e `dock_icons_wanted` (o ícone "Todos os apps" é um glifo Lucide do atlas de texto já carregado, não consome slot no atlas de ícones de apps).
  3. Ao remover ou desinstalar um favorito, `all_apps_dock_index` é ajustado/limitado a `<= favorites.len()`. No editor ("Ordem no Dock") e no `dock_move`, o item participa da reordenação (`◀`/`▶`), mas não oferece ação de remoção.
- **Retrocompatibilidade e reparo**: arquivos `config.v1` antigos sem essas chaves carregam com `blue`, `tabs_visible = true`, `tabs_align = left`, `tabless_mode = slide`, `all_apps_dock_index = None` — comportamento idêntico ao atual. Valores inválidos são reparados para o padrão (como já ocorre com `wallpaper`). `Restaurar padrão` no editor reseta todos esses campos junto com favoritos, formato dos ícones e papel de parede.

### 3. Navegação, foco e GPU para os 5 modos (100% Rust, zero Java)

#### Barra superior (`topbar.rs`)
- **`Com abas` + `À esquerda` (`tabs_visible = true, tabs_align = left`)**: inalterado (`TABS_X = 236.0`).
- **`Com abas` + `Centralizadas` (`tabs_visible = true, tabs_align = center`)**: `BarLayout::new` calcula `total = home_width + TAB_GAP + apps_width` e posiciona o primeiro rótulo em `(CANVAS_WIDTH - total) / 2.0` (centrado em `x = 640.0`), com clamp de segurança para manter folga mínima após o wordmark `"Lume"` e antes de `INPUTS`/`GEAR` em qualquer idioma e formato de relógio (12h/24h).
- **`Sem abas` (`tabs_visible = false`)**: `TopBar::draw` não desenha abas nem sublinhado; `topbar::row` inclui apenas `[INPUTS?, GEAR]` com `entry_up` apontando para `INPUTS` (se disponível) ou `GEAR`. `LumeApp::horizontal` só aciona `select_tab` quando `tabs_visible` é `true`.

#### Modo `Descer` (`tabs_visible = false, tabless_mode = slide`)
- `Screen::Main(Tab::Home)` mantém o papel de parede vivo (`dimmed = false`), o Dock na posição padrão (`dock_bottom`) e uma dica discreta abaixo/junto ao Dock (`v Todos os apps`, glifo chevron + label em 16–18 px `--text-muted`).
- **Transição**: pressionar `Baixo` com foco no Dock muda para `Screen::Main(Tab::Apps)` com animação vertical/fade de **250 ms** (`--duration-medium1` + `--easing-standard-decelerate`, via novo estado em `Anim`), focando o 1º tile da grade e desenhando o papel de parede escurecido (`dimmed = true`).
- **Retorno**: na grade (`Screen::Main(Tab::Apps)`), `Back` ou `Cima` na 1ª linha da grade retorna para `Screen::Main(Tab::Home)` (focando o Dock) com a mesma transição de 250 ms. Na `Home`, `Cima` no Dock acessa a TopBar (`INPUTS`/`GEAR`).

#### Modo `Espiada` (`tabs_visible = false, tabless_mode = peek`)
- **Tela unificada**: `Screen::Main(Tab::Home)` renderiza TopBar + Dock + grade de 6 colunas em uma única viewport rolável, sem troca de `Screen`.
- **Geometria na régua (canvas `1280x720`)**:
  - Dock ancorado em `y = 524..620` (`bottom = 620.0`, 96 px de altura; em 720p/1080p respeita `safe_area` e a régua de 4 unidades).
  - 1ª linha da grade de apps inicia em `GRID_TOP_PEEK = 672.0` (coordenada de conteúdo), expondo ~48 px da borda superior dos tiles abaixo do Dock como convite visual para descer.
- **Foco unificado**: `update_focus` recebe `[topbar_row, dock_row, ...grid_rows]`. Todos os `Focusable` são registrados em **coordenadas de conteúdo** (sem `-scroll_y`). Ao desenhar e ancorar o `ContextMenu` (`Origin::Dock` vs `Origin::Grid`), aplica-se `-current_scroll_y()`.
  - Foco na TopBar ou no Dock força `scroll_to(0.0)`.
  - Foco em qualquer linha da grade aciona `reveal_vertical` com `VIEW_TOP = 88.0`, rolando suavemente a grade para baixo da TopBar e levando o Dock para fora do topo.
- **Escurecimento progressivo sem camada translúcida full-screen**:
  - Em `LumeApp::draw`, no modo `Espiada`, o fator de escurecimento varia continuamente com a rolagem: `dim_amount = (self.current_scroll_y() / PEEK_DIM_DISTANCE).clamp(0.0, 1.0)` (ex.: `PEEK_DIM_DISTANCE = 320.0`, atingindo escurecimento total assim que a 1ª linha da grade sobe para o topo).
  - `Painter::wallpaper_at` recebe `dim_amount: f32` (em vez de `bool`: `0.0` = vivo, `1.0` = `TEXT_SCREEN_DIM.a / 255.0`), escalando a intensidade do próprio quad do papel de parede e das estrelas sobre o `--bg` limpo no início do quadro. **Custo extra de overdraw: zero.**
- **Batching e Culling (60 fps)**:
  - O Dock é submetido a culling vertical (não é desenhado se `dock_bar.bottom() - scroll_y < CLIP_TOP`).
  - A grade mantém exatamente os **3 passes separados** (boxes -> images -> text) sob `push_clip(CLIP_TOP..view_bottom)`, preservando `<= 1` text batch na grade e o teste `apps_grid_does_not_split_text_batch_around_lume_tile`.

#### Modo `No Dock` (`tabs_visible = false, tabless_mode = dock`)
- `Screen::Main(Tab::Home)` mantém papel de parede vivo (`dimmed = false`) e Dock na posição padrão (`y = 584..680`).
- `dock_items()` insere `DockItem { id: ids::DOCK_ALL_APPS, package: None, ... }` no índice `all_apps_dock_index.unwrap_or(favorites.len()).min(favorites.len())`. O item desenha fundo sólido liso em `p.primary()` (com borda sutil `contrast_ring` quando `Preto` estiver ativo) e grade 3×3 de pontos circulares em `p.on_primary()` via SDF (centralizada em `Round`/`Square`; à esquerda do rótulo traduzido `"Apps"` em `Banner` 16:9), participa da Magnification, da bolha de nome e de `dock_move` (reordenação por menu de contexto e no editor), sem opção de remoção.
- `OK` em `ids::DOCK_ALL_APPS` abre `Screen::Main(Tab::Apps)` usando `--tab-fade` (180 ms), TopBar sem abas e foco no 1º tile. `Back` na grade retorna para `Screen::Main(Tab::Home)` com foco restaurado em `ids::DOCK_ALL_APPS`.

#### `Home`, `Back`, `ReturnPoint`, `back_from_settings()` e `splash_images`
- **`ReturnPoint`**: já armazena `{ screen, focus, scroll_y }`. Funciona sem alteração estrutural em todos os 5 modos (restaura foco no Dock ou na linha rolada da grade ao retornar de um app aberto pelo Lume).
- **`Home` com Lume visível (`go_home`)**: retorna para `Screen::Main(Tab::Home)`, `scroll_to_top()` e `PendingFocus::FirstContent` (1º item do Dock).
- **`Back`**:
  - Em `Espiada` com foco na grade ou `scroll_y > 0`: rola para o topo e foca o 1º item do Dock (nunca fecha o Lume).
  - Em `Descer` na grade (`Screen::Main(Tab::Apps)`): volta para `Screen::Main(Tab::Home)` com foco no 1º item do Dock.
  - Em `No Dock` na grade (`Screen::Main(Tab::Apps)`): volta para `Screen::Main(Tab::Home)` com foco em `ids::DOCK_ALL_APPS`.
- **`back_from_settings()`**: quando `tabs_visible == false && tabless_mode == TablessMode::Peek`, volta para `Screen::Main(Tab::Home)` com `PendingFocus::Id(ids::GRID_SETTINGS)` (deixando `reveal_vertical` rolar até o tile Lume); nos demais modos, volta para `Screen::Main(Tab::Apps)` com `PendingFocus::Id(ids::GRID_SETTINGS)`.
- **`splash_images`**: no modo `Espiada`, coleta tanto os ícones do Dock quanto os tiles visíveis a partir de `GRID_TOP_PEEK` (respeitando `self.scroll_y`), garantindo que a 1ª linha visível na espiada já esteja pronta antes do reveal.

#### Editor (`Configurações › Tela Início › Painel APARÊNCIA`)
- Abaixo do seletor de papel de parede e do divisor, o painel `APARÊNCIA` passa a conter:
  1. **Linha `Cor de destaque`**: rótulo + nome da cor em foco/ativa à esquerda e os **10 swatches circulares** à direita (`40×40` ou `44×44` na régua de 4, com check em `--on-primary` no ativo e anel de foco `3 px` na própria cor + borda de contraste no `Preto`). Navegar com `Esquerda`/`Direita` atualiza `preview_accent` (live preview imediato); `OK` persiste em `config.v1`; sair da linha (`Cima`/`Baixo`/`Back`) sem `OK` limpa `preview_accent` e restaura a cor salva.
  2. **Menu condicional em 2 níveis**:
     - Nível 1 — `Abas na barra superior`: `Com abas` | `Sem abas`.
     - Nível 2 — se `Com abas`: `Posição das abas` (`À esquerda` | `Centralizadas`); se `Sem abas`: `Acesso aos apps` (`Descer` | `Espiada` | `No Dock`).
  3. **Linha `Formato dos favoritos`**: `.editor-row` (`64 px`) com 3 chips de `40 px` alinhados à direita em `x = 1160`: `Redondo` *(padrão)* | `Quadrado` | `Retangular`, substituindo o antigo toggle binário `Favoritos redondos`.
  4. Quando `Sem abas` + `No Dock` está ativo, a seção de favoritos exibe o item `"Todos os apps"` na ordem configurada com botões `◀`/`▶` habilitados e botão remover desabilitado.

## Risks / Trade-offs

- **Risco 1 — Queda de quadros na AIWA 720p `low_ram` durante a rolagem do modo `Espiada`**
  - *Mitigação*: dimming progressivo feito exclusivamente no multiplicador de intensidade de `Painter::wallpaper_at` (zero camada translúcida full-screen); `Shell::is_busy` bloqueia icon loading durante `Anim::ScrollY`, `Anim::DockPosition` e `Anim::DockAmplitude`; culling vertical do Dock e da grade; manutenção rigorosa dos 3 passes da grade (`<= 1` text batch).
- **Risco 2 — Largura do Dock com 13 itens (12 favoritos + "Todos os apps") no modo `No Dock` + `MOVE_ROOM` de reordenação**
  - *Cálculo*: em repouso, 13 ícones ocupam `13*64 + 12*16 + 32 = 1056 px` (`~1146 px` com Magnification no pico, dentro da safe area de `1152 px` e do canvas de `1280 px`). Porém, durante `dock_move` (`+2 * MOVE_ROOM = +48 px`), chega a `1194 px` (passando ligeiramente da safe area de `1152 px`, ainda dentro de `1280 px`).
  - *Mitigação*: em `dock::layout_with_room`, quando `count > 12` e a largura da barra exceder `safe_area.w`, reduzir proporcionalmente apenas o `GAP` (ex.: de `16` para `12 px`) enquanto houver 13 itens, mantendo todos os ícones e setas dentro da área segura em 720p e 1080p.
- **Risco 3 — Contraste de `Preto` e `Branco` em superfícies extremas**
  - *Mitigação*: `Branco` troca o preenchimento de foco em cartões claros (`.menu-item:focus`, `.menu-button:focus`) para `#1D2730` com texto `#F1F4F9`; `Preto` usa `--on-primary` claro (`#F1F4F9`), borda externa de contraste `rgba(255,255,255,0.45)` em focos sobre superfícies escuras e halo claro sutil no Splash. Testes unitários em `theme.rs` validarão todas as 10 cores contra os limiares de contraste (WCAG `>= 4.5:1` texto, `>= 3:1` contorno).
- **Risco 4 — Transição visual da *starting window* do Android (`#0C171F`) para o Splash colorido**
  - *Mitigação*: sem tocar Java/XML, o primeiro quadro do Splash inicia exatamente em `#0C171F` e interpola suavemente para `accent.splash_bg` durante `--splash-logo-in` (300 ms), eliminando qualquer flash brusco na abertura a frio.
- **Risco 5 — Consumo de memória (`<= 32 MB` Private Dirty)**
  - *Mitigação*: `Theme` e `AccentPalette` são `Copy` sem heap; o botão "Todos os apps" usa glifo Lucide já presente no atlas de texto (zero slot extra em `AppIcons`); nenhum buffer novo é alocado.

## Migration Plan

- Nenhuma migração destrutiva ou bump de versão do arquivo `config.v1` (`VERSION = 1`).
- Instalações existentes sem as novas chaves continuam abrindo com `Azul Lume` (`#4DB3FF`) e `Abas à esquerda`, idêntico ao comportamento atual.
- Em caso de rollback para uma versão anterior do APK, o parser `storage::Kv` ignora as novas chaves desconhecidas sem perder favoritos nem papel de parede.
- Ao final da implementação, revisar a tela **Sobre o Lume** (conforme regra do projeto) e validar os orçamentos na TCL 55C715, na AIWA 720p e no emulador Android TV 9.

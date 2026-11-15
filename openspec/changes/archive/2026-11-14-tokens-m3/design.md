# Design

## Context

Motivação em `proposal.md`. Estado atual que molda a abordagem:

- O tema (`themes/lume/theme.css`) é lido em tempo de build por `crates/lume-css` e convertido em código Rust por `crates/lume-core/build.rs`. O parser já entende `cubic-bezier(...)`, `letter-spacing` e `box-shadow`, mas **não** entende `line-height`.
- A sombra (`render/list.rs`, `RectKind::Shadow`) é um único retângulo com borda suave calculada no shader, sem passe de desfoque. O custo é só a área a mais preenchida. Os 1,5% de quadros perdidos na grade **não** devem ser atribuídos a ela sem medir.
- Há cinco sombras: `.dock` (0 16 40, 45%), `.tile:focus` (`--focus-shadow`, 0 12 32, 45%), `.menu` (0 4 24, 45%, que também vale para os cartões claros de Mapeamento de controle), `.toast` (0 8 24, 40%) e a do mini-card de papel de parede, fixa em `app/editor.rs` (raio 12, desfoque 16, alfa 160), o que fere a regra de fonte única.
- Todas as cores escuras de hoje já estão no mesmo matiz HCT (cerca de 245, o mesmo do `--primary`, 247). Muda só o croma: as superfícies têm de 10 a 13, foco, controle e contorno têm de 16 a 18, e os textos de 5 a 11. A paleta tonal só torna explícito o que já existia.
- Decisões do autor que esta change preserva: `--primary` `#4DB3FF` em todo foco, sublinhado das abas, menu claro, Plus Jakarta Sans, fade de aba de 180 ms, reordenação do dock em 240 ms, sem sombra nos papéis de parede e contraste da barra superior fora de discussão.

## Goals / Non-Goals

**Goals:**
- Toda cor neutra e de perigo sai de um tom de uma paleta declarada. A regra de tom garante o contraste, e um teste confere.
- Forma, movimento, elevação e tipografia viram tokens com nome, e os componentes passam a usar esses nomes.
- O foco dos tiles e dos mini-cards fica só com o anel e a escala.

**Non-Goals:**
- Cor dinâmica pelo papel de parede (Material You).
- Componentes do M3 ou do `tv-material`: abas em pílula, carrossel, menu escuro e camadas de estado translúcidas.
- Trocar a fonte, mudar o azul primário ou mudar as cores de marca dos tiles (`--brand-*`, `--tile-*`, `--dot-*`).
- Calcular paletas em tempo de execução.

## Decisions

### D1. Quatro paletas de neutros e erro, mais o primário fixo

| Paleta | Matiz | Croma | Tokens (tom) |
|---|---|---|---|
| `surface` | 245 | 12 | `--bg` 7, `--surface` 10, `--surface-row` 13, `--surface-icon` 14, `--control-disabled` 16, `--menu-text` 15 |
| `surface-variant` | 246 | 18 | `--surface-focus` 19, `--control` 24, `--border` 27 |
| `neutral` | 240 | 6 | `--text`/`--secondary` 96, `--text-soft` 90, `--text-muted` 75, `--text-disabled` 41, `--menu-surface` 96, `--menu-text-muted` 44 |
| `error` | 25 | 84 | `--danger-bg` 20, `--danger-fg` 80, `--menu-danger` 40 |

`--primary` continua `#4DB3FF` (tom 70, matiz 247), declarado como semente e não regenerado. `--on-primary` é `--bg`.

Valores calculados para referência (`materialyoucolor`):

```
surface         7 #0C171F  10 #121D25  13 #18232C  14 #1B252E  15 #1D2730  16 #1F2932
surface-variant 19 #1E303F  24 #293B4B  27 #304252
neutral         96 #F1F4F9  90 #DFE3E8  75 #B5B9BE  44 #64696D  41 #5D6165
error           20 #690005  40 #BA1A1A  80 #FFB4AB
```

Contrastes conferidos: `--text-muted` sobre `--surface` 8,7:1; `--menu-text-muted` sobre `--menu-surface` 5,0:1; `--menu-text` 13,8:1; `--menu-danger` 5,9:1; `--danger-fg` sobre `--danger-bg` 7,7:1.

- *Por que separar `surface-variant`?* Com croma 12, o `--surface-focus` ficaria cinza (`#252F39`, contra `#1F3040` hoje), e o foco das linhas perderia o azul. No M3, o `surface-variant` e o `outline` também saem de uma paleta à parte.
- *Por que textos com croma 6?* O texto de hoje é quase branco (croma 5), e croma 12 no tom 96 azularia o texto.
- *Por que o `error` padrão do M3?* `--menu-danger` `#B3261E` já é praticamente o `error40` de referência do M3. O `--danger-fg` vai do rosado `#FFB4BE` para `#FFB4AB`, e o `--danger-bg` de `#4A2630` (vinho) para `#690005` (vermelho escuro). Esta última é a maior mudança visível da change. Fica registrada e só pode ser revertida por decisão do autor.
- *Alternativa descartada:* manter os hex de hoje e só dar nomes do M3. Isso não garante o contraste por regra, que é o ganho principal.

### D2. Hex gravado no tema, gerado por script

`scripts/tonal-palette.py` (via `uv`, com `materialyoucolor`) lê as sementes e os tons de um bloco comentado no topo do `theme.css` e reescreve os hex. O tema guarda **os hex** e, ao lado de cada token, um comentário `/* surface T10 */`. O build não ganha dependência nem passo novo, e a tela Sobre não muda. Um teste de tema confere a regra de contraste por tom sobre os hex gravados.

- *Alternativa:* o crate `material-colors` no `build.rs`. Descartada porque traz dependência de terceiros para gerar 20 números que quase nunca mudam.

### D3. Apelidos de papel do M3

Os nomes atuais ficam (as telas e a spec usam esses nomes). O tema ganha apelidos com os papéis do M3, para documentar a correspondência e servir a componentes novos: `--surface-container-lowest` → `--bg`, `--surface-container-low` → `--surface`, `--surface-container` → `--surface-row`, `--surface-container-high` → `--surface-focus`, `--on-surface` → `--text`, `--on-surface-variant` → `--text-muted`, `--outline` → `--border`, `--error` → `--menu-danger`, `--error-container` → `--danger-bg`, `--on-error-container` → `--danger-fg`, `--inverse-surface` → `--menu-surface`, `--inverse-on-surface` → `--menu-text`. Um componente novo escolhe pelo papel do M3.

### D4. Forma

`--shape-md: 12px` (e `--radius` passa a ser apelido dele), `--shape-lg: 20px`, `--shape-xl: 28px` e `--shape-full` (metade da altura, para chip, keycap e botão redondo). Os componentes passam a usar esses nomes. Nenhum raio muda.

### D5. Movimento

Curvas do M3 com os valores de referência:

```
--easing-standard               cubic-bezier(0.2, 0, 0, 1)
--easing-standard-decelerate    cubic-bezier(0, 0, 0, 1)
--easing-standard-accelerate    cubic-bezier(0.3, 0, 1, 1)
--easing-emphasized-decelerate  cubic-bezier(0.05, 0.7, 0.1, 1)
--easing-emphasized-accelerate  cubic-bezier(0.3, 0, 0.8, 0.15)
```

Durações: `--duration-short1..4` 50/100/150/200, `--duration-medium1..4` 250/300/350/400, `--duration-long1..4` 450/500/550/600.

Mapeamento: o foco (`--focus-time`) passa a ser `--duration-short3` com `--easing-standard-decelerate`. O ícone que entra, o menu de contexto e o logo do splash, que são coisas que entram, usam `--easing-emphasized-decelerate`. O splash usa `--duration-medium2` (300) e `--duration-long4` (600). A revelação a quente do splash (360) cai em `--duration-medium3` (350). As exceções do autor ficam como tokens de uso fora da escala, com comentário: `--tab-fade: 180ms` e `--dock-reorder: 240ms` (ease-in-out). A Magnification do dock não muda.

- *Por que `decelerate` no foco, e não a curva `standard`?* O foco é uma resposta imediata ao controle remoto. A desaceleração começa rápido e mantém a sensação de 150 ms atual. A `standard` atrasaria o início.
- *Molas (M3 Expressive):* ficam fora desta change. Seriam um passo à parte no dock.

### D6. Elevação e sombras

`--elevation-1: 0 4px 24px rgba(0,0,0,.45)` (cartões claros: menu, modal, diálogo), `--elevation-2: 0 8px 24px rgba(0,0,0,.40)` (aviso), `--elevation-3: 0 16px 40px rgba(0,0,0,.45)` (dock). Os níveis mantêm os valores atuais, ganhando só o nome. O `.tile:focus` perde o `box-shadow` e o token `--focus-shadow` é removido. `editor.rs` deixa de chamar `list.shadow` no mini-card.

- *Por que não trocar sombra por superfície mais clara no tile?* A superfície do tile é o banner do app, então não há fundo para clarear. O anel de 4 px e a escala 1,05 já separam o tile em foco.

### D7. Tipografia em pacote

| Nível | Tamanho | Altura de linha | Espaçamento | Referência M3 |
|---|---|---|---|---|
| título | 40 | 48 | -0,8 (`.page-title` hoje) | display small (36/44), ajustado |
| marca | 28 | 36 | -0,5 (`.brand` hoje) | headline medium 28/36 |
| primário | 22 | 28 | 0 | title large 22/28 |
| corpo | 20 | 28 | 0 | entre body large e title large |
| rótulo | 18 | 24 | 0 | title medium/label large, ajustado |
| legenda | 16 | 24 | 0, e 1,2 em caixa alta (`.card-heading`) | body large 16/24 |

Tokens `--line-title` ... `--line-caption` e `--tracking-*`. Todas as alturas de linha são múltiplas de 4, para cair na régua. O espaçamento atual da marca e do título vira o token do nível. Quem usa caixa alta (cabeçalho de cartão) usa `--tracking-caps`.

`line-height` entra no parser (`lume-css`), no `build.rs` e no estilo computado. O layout de texto usa a altura de linha **só onde há várias linhas** (texto da tela Sobre, licenças, quebras), porque as linhas de texto único já são posicionadas pela linha-base, na régua. Assim nenhuma posição de tela muda. A altura de linha atual das telas de várias linhas precisa ser conferida, e quando ela diferir da tabela, a spec da tela é atualizada junto.

### D8. Referências

A spec `design-system` ganha um requisito com as fontes do M3 usadas e as adaptações do Lume, para quem retomar saber de onde vem cada regra e o que foi recusado de propósito.

## Risks / Trade-offs

- [`--danger-bg` muda de vinho para vermelho escuro] → é a maior mudança visível. Comparar as capturas do botão de perigo e do diálogo antes e depois. Se o autor recusar, a paleta `error` ganha um tom mais baixo ou um croma menor, sem mudar a regra.
- [Tile sem sombra pode parecer "chapado" sobre papéis claros] → comparar capturas da grade na Aurora e no papel mais claro. O anel de 4 px é o padrão de TV.
- [`line-height` mexer em telas de várias linhas] → teste de régua e capturas da tela Sobre antes e depois. Se a altura atual for outra, ajustar a spec da tela, não o token.
- [Curvas novas mudam a sensação do foco] → conferir no emulador e na TCL. A duração não muda.
- [Ganho de quadros pode ser nulo] → a medida na TCL registra o resultado como ele vier, sem prometer melhora.

## Migration Plan

Mudança só de tema e de desenho, sem dados persistidos. Reverter é desfazer o commit squash.

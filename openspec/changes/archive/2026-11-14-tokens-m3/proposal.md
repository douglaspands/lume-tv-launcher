# Proposal

## Why

Os valores visuais do Lume foram escolhidos um a um, a cada change: oito superfícies de ardósia com hex escritos à mão, durações e curvas soltas (150, 180, 240, 300, 360, 600 ms), raios sem nome e tipografia que fixa tamanho e peso, mas não altura de linha. Toda tela nova precisa adivinhar um valor "entre" os que já existem. O Material Design 3 (https://m3.material.io/) resolve isso com sistemas de tokens gerados por regra: paleta tonal, escala de forma, tokens de movimento, elevação e escala tipográfica. Adotar as **regras** do M3 (não a aparência nem os componentes, que são Compose) dá consistência e contraste garantido por construção, e mantém a identidade do Lume.

## What Changes

- **Cor por paleta tonal (HCT)**: as superfícies, textos, cartão claro e cores de perigo passam a ser tons de três paletas (neutra, neutra-variante e de erro), com os nomes de papel do M3 (`surface-container-*`, `on-surface`, `outline`...) como apelidos. Os hex mudam um pouco, o bastante para cair no tom exato. O azul `--primary` `#4DB3FF` **não muda** (decisão do autor), e também não entra cor dinâmica pelo papel de parede.
- **Contraste por regra de tom**: 50 tons de diferença garantem 4,5:1 e 40 tons garantem 3:1. A regra é conferida por teste do tema.
- **Forma**: os raios ganham nome na escala do M3: `--shape-md` 12, `--shape-lg` 20, `--shape-xl` 28 e `--shape-full` (metade da altura).
- **Movimento**: curvas com nome (`--easing-standard`, `--easing-emphasized-decelerate`, `--easing-emphasized-accelerate`...) e durações em degraus (`--duration-short1..4`, `--duration-medium1..4`, `--duration-long*`). Os tokens de uso (`--focus-time`, splash, ícones) passam a apontar para eles. As durações fixadas pelo autor (fade de aba de 180 ms, reordenação do dock em 240 ms) são mantidas como exceções registradas.
- **Elevação**: três níveis (`--elevation-1..3`) para as camadas que flutuam: aviso (`.toast`), dock e cartões claros (menu de contexto, modal e diálogo de Mapeamento de controle).
- **BREAKING (visual)**: o tile em foco (`.tile:focus`) e o mini-card de papel de parede em foco **perdem a sombra**. O foco fica com o anel azul e a escala, como no padrão de TV do Google. A sombra do mini-card, hoje fixa no código, sai junto.
- **Tipografia em pacote**: cada um dos 6 níveis passa a ter tamanho, peso, altura de linha e espaçamento de letras (`--line-*`, `--tracking-*`). A fonte continua a Plus Jakarta Sans. O tema passa a aceitar `line-height`, que hoje o parser de CSS não lê.
- **Referências do M3 no design system**: a spec registra as fontes (páginas do M3 e do Design for TV) e as adaptações do Lume.

## Capabilities

### New Capabilities
<!-- nenhuma -->

### Modified Capabilities
- `design-system`: tokens de cor passam a vir de paletas tonais com regra de contraste por tom; a tipografia ganha altura de linha e espaçamento por nível; foco e movimento passam a usar tokens de curva e duração; as superfícies ganham níveis de elevação e o tile em foco perde a sombra; os mini-cards do editor perdem a sombra no foco; novos requisitos de escala de forma e de referências ao M3.

## Impact

- `themes/lume/theme.css`: paletas, apelidos de papel, `--shape-*`, `--easing-*`, `--duration-*`, `--elevation-*`, `--line-*`, `--tracking-*`; remoção do `box-shadow` de `.tile:focus`.
- `crates/lume-css` (parser) e `crates/lume-core/build.rs`: nova propriedade `line-height`.
- `crates/lume-core`: o layout de texto usa a altura de linha do tema; `app/editor.rs` deixa de desenhar a sombra fixa do mini-card; testes de tema para contraste por tom e tokens.
- `openspec/specs/design-system/spec.md` (via delta), `docs/theme-format.md` (novas propriedades e tokens) e `docs/device-notes.md` (medida antes/depois na TCL).
- Sem dependência nova em tempo de execução. Os tons são calculados uma vez e gravados como hex no tema, com o script de geração em `scripts/` (Python via `uv`). Sem efeito esperado no APK nem no PSS.

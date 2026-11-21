# Proposal

## Why

Hoje todos os destaques visuais do Lume (contornos de foco, sublinhado ativo, botões primários, logo e tela de carregamento) são fixos em azul (`#4DB3FF`), e a navegação da tela inicial só existe no formato de abas (`Início` / `Apps`) alinhadas à esquerda na barra superior. Dar ao usuário opções curadas de cor de destaque e diferentes modos de organização da tela inicial — mantendo a leveza, o contraste a 3 metros e o orçamento de quadros das TVs — torna o launcher mais pessoal e adaptável a diferentes preferências de uso.

## What Changes

- **Paleta de 10 cores de destaque configuráveis**: adiciona 10 presets calibrados para TV (`Azul Lume` padrão `#4DB3FF`, `Aurora` `#00C0B2`, `Esmeralda` `#57C071`, `Âmbar` `#EC9A00`, `Coral` `#FF8163`, `Rosa` `#FA7ABC`, `Violeta` `#AC96FF`, `Vermelho` `#FF5449`, `Branco` `#F1F4F9` e `Preto` `#11151A`), selecionáveis em `Configurações › Tela Início › Painel APARÊNCIA` com pré-visualização ao vivo no D-pad (`OK` salva; sair sem `OK` reverte).
- **Logo vazado e Splash imediatamente na cor escolhida**: o logo do Lume (barra superior, tile "Lume", breadcrumb, Splash e página do celular) acompanha a cor de destaque escolhida com o **"L" e o ponto transparentes (vazados)**, deixando aparecer o fundo/papel de parede por trás; o Splash exibe diretamente o fundo escuro T7 da cor escolhida desde o 1º quadro (sem passar pelo azul padrão `#0C171F` ao apertar `Home`), com `windowBackground` neutro escuro (`#0A0D10`) na abertura a frio.
- **Regras de contraste para Branco e Preto**: garante legibilidade de foco e texto a 3 metros em todas as superfícies — `--on-primary` claro (`#F1F4F9`) e borda externa sutil de contraste em fundos escuros para `Preto`, e foco escuro no cartão claro do menu de contexto para `Branco`.
- **Menu condicional de navegação em 2 níveis (`Configurações › Tela Início › Painel APARÊNCIA`)**:
  - **Nível 1 — `Abas na barra superior`**: `Com abas` (padrão) ou `Sem abas`.
  - **Nível 2 (quando `Com abas`) — `Posição das abas`**: `À esquerda` (padrão atual) ou `Centralizadas` (no centro da barra superior).
  - **Nível 2 (quando `Sem abas`) — `Acesso aos apps`**:
    - `Descer`: dock limpo no Início com dica discreta `v Todos os apps`; 1 clique `Baixo` abre a grade em tela cheia (`250 ms`).
    - `Espiada`: dock mais baixo (`y=524..620`) com a borda superior (~48 px) da 1ª linha de apps visível embaixo; 1 clique `Baixo` rola suavemente para a grade com escurecimento progressivo do papel de parede.
    - `No Dock`: botão **"Todos os apps"** dentro do próprio Dock (slot extra além dos até 12 favoritos), reordenável na lista de favoritos mas sem opção de remoção.
- **Formato dos ícones no Dock (3 opções em `Configurações › Tela Início › Painel APARÊNCIA`)**: substitui o toggle binário `"Favoritos redondos"` pelo seletor **`Formato dos favoritos`** com 3 escolhas — **`Redondo`** *(padrão)*, **`Quadrado`** (cantos arredondados) e **`Retangular`** (banners 16:9 dos apps na TV, iguais aos tiles da grade de Apps, com redução proporcional para caber até 12 favoritos + `"Todos os apps"` dentro da área segura).
- **Persistência retrocompatível**: novas chaves opcionais em `files/config.v1` com padrões idênticos ao comportamento atual (`blue`, `tabs`, `left`, `dock_shape=round` com fallback da chave legada `round_favorites`), restauradas por `Restaurar padrão`.

## Capabilities

### New Capabilities
<!-- Nenhuma nova capacidade isolada: as mudanças evoluem capacidades já existentes do Lume. -->

### Modified Capabilities
- `design-system`: adiciona a paleta de 10 cores de destaque (com pares de contraste para `Branco` e `Preto`), o logo e o fundo T7 do Splash acompanhando `--primary`, a variante de abas centralizadas, as métricas dos modos `Descer`, `Espiada` e `No Dock`, e o seletor de 3 formatos do dock (`Redondo`, `Quadrado`, `Retangular` 16:9).
- `theming`: permite sobrescrever dinamicamente em tempo de execução os tokens de cor primária/destaque (`--primary`, `--on-primary`, foco em cartão claro, borda de contraste e fundo T7 do Splash) sem recompilar o tema nem criar texturas extras.
- `launcher-shell`: suporta os 5 estilos de navegação (`Com abas` à esquerda ou centralizadas; `Sem abas` nos modos `Descer`, `Espiada` e `No Dock`), persistência em `config.v1`, foco na barra superior sem abas e fluxo de `Home`, `Back` e `ReturnPoint`.
- `home-screen`: adapta a tela Início aos modos `Descer`, `Espiada` (com escurecimento progressivo do papel de parede no próprio desenho, sem camada translúcida full-screen), `No Dock` (com o botão fixo e reordenável "Todos os apps") e aos 3 formatos de ícones do dock (`Redondo`, `Quadrado`, `Retangular`).
- `apps-screen`: integra a grade de 6 colunas abaixo do Dock nos modos `Descer` e `Espiada`, o retorno da grade para o Dock nos modos sem abas e o retorno das Configurações do Lume (`ids::GRID_SETTINGS`) quando as abas estão ocultas.
- `app-catalog`: permite exibir no dock o ícone do app (`Redondo` ou `Quadrado`) ou o banner 16:9 de TV já em cache (`Retangular`), sem alocação extra de textura.
- `home-editor`: adiciona ao painel `APARÊNCIA` o seletor de 10 swatches de cor com pré-visualização ao vivo, o seletor condicional em 2 níveis de navegação da tela inicial, o seletor de 3 opções **`Formato dos favoritos`** (`Redondo` | `Quadrado` | `Retangular`) no lugar do toggle binário, e o suporte a reordenar sem remover o item "Todos os apps" no modo `No Dock`.

## Impact

- **Código Rust (100% Rust, zero Java)**: `crates/lume-core` (`theme.rs`, `home/config.rs`, `shell/mod.rs`, `app/{mod,painter,topbar,home,apps,dock,editor,settings,splash,page,context_menu,keymap,receive}.rs`, `tools/common.rs`, `companion/page.rs`), strings em `strings.xml` (PT-BR e EN).
- **Performance e memória**: zero alocação extra de textura/GPU para troca de cor; escurecimento progressivo no modo `Espiada` feito no próprio `Painter::wallpaper_at` sobre o clear `--bg`, respeitando os orçamentos da spec `resource-efficiency` (TCL 55C715 e AIWA 720p `low_ram`).
- **Compatibilidade**: 100% retrocompatível com instalações existentes (`config.v1` sem as novas chaves mantém Azul Lume + Com abas à esquerda).

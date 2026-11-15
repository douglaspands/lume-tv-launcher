# Tasks

## 1. Linha de base

- [x] 1.1 Capturar, antes de qualquer mudança, `home`, `apps` (com foco num tile), `settings`, `about` e `editor` em 1920x1080 com `lume-desktop` e guardar fora do repositório para comparação; verificar que as 5 capturas existem
- [x] 1.2 Medir na TCL os quadros de 33 ms ou mais na navegação da grade e do dock e o PSS em primeiro plano (procedimento de `docs/device-notes.md`); verificar que os números estão anotados para a tarefa 7.2

## 2. Paletas tonais e cor

- [x] 2.1 Criar `scripts/tonal-palette.py` (`uv`, `materialyoucolor`) que lê as sementes e os tons declarados no tema e reescreve os hex dos tokens neutros e de perigo (design D1/D2); verificar que `uv run scripts/tonal-palette.py --check` passa sem diferenças depois da reescrita
- [x] 2.2 Aplicar no `theme.css` as quatro paletas e os tons de D1, com o comentário `/* <paleta> T<tom> */` em cada token, `--on-primary` igual a `--bg` e `--primary` intacto; verificar com `cargo test --workspace`
- [x] 2.3 Adicionar os apelidos de papel do M3 (D3); verificar com um teste do tema que cada apelido resolve para a mesma cor do token de origem
- [x] 2.4 Adicionar o teste de contraste por tom (50 tons para texto, 40 para contorno) sobre os pares usados pelos componentes, mantendo `#[ignore]` nos dois testes de contraste da barra superior; verificar que ele passa e que falha ao baixar `--text-muted` para T55
- [x] 2.5 Atualizar `docs/theme-format.md` com as paletas, os comentários de tom, os apelidos e o script; verificar que o comando documentado roda como escrito

## 3. Forma e elevação

- [x] 3.1 Adicionar `--shape-md/lg/xl/full`, tornar `--radius` apelido de `--shape-md` e trocar os raios dos componentes pelos tokens; verificar com `the_component_boxes_are_on_the_ruler` e capturas sem diferença de raio
- [x] 3.2 Adicionar `--elevation-1/2/3` com os valores atuais e usá-los em `.menu`, nos cartões claros de Mapeamento de controle, em `.toast` e em `.dock`; verificar com os testes de tema de sombra do dock e do menu
- [x] 3.3 Remover o `box-shadow` de `.tile:focus` e o token `--focus-shadow`, ajustando o teste do tema que esperava a sombra no tile em foco; verificar com `cargo test -p lume-core` e captura de `apps` com foco num tile sem sombra
- [x] 3.4 Remover a sombra fixa do mini-card em foco em `app/editor.rs`; verificar com `cargo test --workspace` e captura de `editor` com foco num mini-card

## 4. Movimento

- [x] 4.1 Adicionar as curvas `--easing-*` e as durações `--duration-*` (D5) e as exceções `--tab-fade` e `--dock-reorder` com comentário de decisão do autor; verificar com `cargo test --workspace`
- [x] 4.2 Apontar `--focus-time`, `--icon-in`, os tokens do splash e a aparição do menu para os tokens de D5; verificar no emulador (`scripts/emulator.sh install debug`) a navegação na grade, o menu de contexto e o splash a frio e a quente — conferido na TCL (o AVD `tv_api28` não existe mais nesta máquina); ver `docs/device-notes.md`
- [x] 4.3 Fazer o build do tema avisar quando uma duração não é um degrau da escala nem uma exceção registrada; verificar com um teste do `lume-core` ou do `lume-css` que uma duração de 170 ms gera o aviso

## 5. Tipografia

- [x] 5.1 Ensinar `line-height` ao `lume-css` (valor em px) e ao `build.rs`, levando o valor até o estilo computado; verificar com um teste de parser em `crates/lume-css/tests/parser.rs` e um teste de tema
- [x] 5.2 Adicionar `--line-*` e `--tracking-*` (D7) e aplicá-los aos componentes de texto, mantendo o espaçamento atual de `.brand`, `.page-title` e `.card-heading`; verificar com `cargo test --workspace`
- [x] 5.3 Fazer o layout de texto de várias linhas (tela Sobre, licenças, quebras) usar a altura de linha do estilo; verificar com o cenário "Texto de várias linhas" num teste e comparar a captura de `about` com a da tarefa 1.1, atualizando a spec da tela se o passo mudar

## 6. Design system e documentação

- [x] 6.1 Conferir que o `theme.css` final corresponde ao delta de `specs/design-system/spec.md` (hex, tons, tokens e referências) e corrigir o que divergir; verificar com `openspec validate tokens-m3 --strict`
- [x] 6.2 Atualizar as capturas do README se a mudança de cor ou de sombra for visível nelas; verificar comparando com as capturas da tarefa 1.1 — a mudança é visível (botões de perigo do editor); por decisão do autor, as 20 capturas são regeneradas pela change `atualize-prints-readme`, depois do merge desta

## 7. Integração

- [x] 7.1 Rodar `cargo test --workspace`, `cargo clippy --workspace --all-targets` e `cargo fmt --all --check`; verificar que estão limpos
- [x] 7.2 Instalar o APK release na TCL e repetir a medida da tarefa 1.2, registrando o antes e o depois em `docs/device-notes.md` (sem prometer melhora); verificar que os orçamentos de `resource-efficiency` continuam atendidos
- [x] 7.3 Mostrar ao autor, lado a lado, as capturas antes e depois do botão de perigo, do diálogo do editor e da grade em foco (riscos do design); verificar que o autor aprovou ou que a paleta foi ajustada — aprovado pelo autor em 2026-11-14 (botão de perigo, diálogo do editor, diálogo de Mapeamento e grade em foco)
- [x] 7.4 Revisar a tela Sobre (Configurações do Lume › Sobre o Lume): não entra crate, fonte nem ícone novo, porque o script de paleta é ferramenta de desenvolvimento; registrar "Sobre revisado, sem mudança" ou atualizar se algo novo entrou; verificar com `cargo run -p lume-desktop -- --screenshot about` — Sobre revisado, sem mudança (a captura de `about` bate com a da tarefa 1.1)

## Workflow follow-up

- `/opsx:archive` para levar o delta à `openspec/specs/design-system/spec.md`.
- Merge squash na `main`.

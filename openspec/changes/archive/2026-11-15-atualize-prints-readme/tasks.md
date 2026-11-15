# Tasks

## 1. Captura e Processamento das Imagens

- [x] 1.1 Identificar/configurar fonte de fallback para cirílico (`LUME_FALLBACK_FONT`) e script de captura para os 5 idiomas (`en`, `pt-BR`, `es`, `id`, `ru`) e 4 telas (`home`, `apps`, `settings`, `editor`) *(A fonte de reserva não é mais necessária: o cirílico vem embutido (Manrope, change `fonte-cirilico`). Capturas com `--favorites 8`, `LUME_NO_SYNTHETIC_ICONS=1`, `--keys right` na Início e `LUME_VERSION_NAME=v0.8.0` nas Configurações, para mostrar a versão da tag em vez do hash `-dirty`; a frase dos READMEs sobre `LUME_FALLBACK_FONT` saiu.)*
- [x] 1.2 Executar a geração dos screenshots com `lume-desktop` e converter para JPEG otimizado em `docs/screenshots/`, verificando que todos os 20 arquivos `.jpg` foram atualizados com peso adequado *(1920x1080 reduzidas para 1600x900, JPEG qualidade 88; de 68 a 112 KB cada, 1,9 MB no total.)*
- [x] 1.3 Inspecionar visualmente as capturas geradas para garantir ausência de artefatos e fidelidade das traduções *(Conferidas. Defeito da interface, fora desta change por decisão do autor: no editor, o nome do papel de parede passa da borda do cartão ("Pegunungan Berbintang" em `id`, "Montanhas Estreladas" em `pt-BR`), e "Mirror Lake" e "Misty Forest" estão sem tradução em `id`; corrigir numa change separada e refazer os prints do editor.)*

## 2. Validação da Documentação e Finalização

- [x] 2.1 Verificar que todos os READMEs (`README.md`, `README.pt-BR.md`, `README.es.md`, `README.id.md`, `README.ru.md`) referenciam corretamente as imagens atualizadas e exibem paridade *(Cada README aponta para as 4 imagens do próprio idioma, todas existentes.)*
- [x] 2.2 Rodar `cargo test --workspace && cargo clippy --workspace --all-targets` e `cargo fmt --all --check` para garantir integridade do repositório *(791 testes passaram; clippy e fmt limpos.)*
- [x] 2.3 Revisar a tela Sobre (Configurações do Lume › Sobre o Lume) com `cargo run -p lume-desktop -- --screenshot about a.png`; sem componentes novos ou alterações de licença/repositório/versão, registrar "Sobre revisado, sem mudança" *(Sobre revisado, sem mudança.)*

## 3. Desativar também as recomendações

- [x] 3.1 Nos 5 READMEs, passo 6 da Opção 2: o último recurso desativa `com.google.android.tvlauncher` e `com.google.android.tvrecommendations`, com uma frase sobre o congelamento do Lume que só o launcher desativado causa
- [x] 3.2 Nos 5 READMEs, "Voltar ao launcher original": `pm enable` dos dois pacotes antes de `adb uninstall app.lume.tv`
- [x] 3.3 Nos 5 READMEs, prompt da Opção 1: novo passo, se o Home ainda abrir o launcher original, pedir confirmação e desativar os dois pacotes juntos, explicando como reverter; manter a regra de não rodar outros comandos sem pedir
- [x] 3.4 Conferir a paridade dos comandos entre os 5 idiomas (`grep -c tvrecommendations README*.md` igual em todos)

## Workflow follow-up

- Arquivar a change com `openspec archive atualize-prints-readme`
- Merge na branch `main` com squash conforme fluxo do projeto

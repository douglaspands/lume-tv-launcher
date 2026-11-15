# Tasks

## 1. Captura e Processamento das Imagens

- [ ] 1.1 Identificar/configurar fonte de fallback para cirílico (`LUME_FALLBACK_FONT`) e script de captura para os 5 idiomas (`en`, `pt-BR`, `es`, `id`, `ru`) e 4 telas (`home`, `apps`, `settings`, `editor`)
- [ ] 1.2 Executar a geração dos screenshots com `lume-desktop` e converter para JPEG otimizado em `docs/screenshots/`, verificando que todos os 20 arquivos `.jpg` foram atualizados com peso adequado
- [ ] 1.3 Inspecionar visualmente as capturas geradas para garantir ausência de artefatos e fidelidade das traduções

## 2. Validação da Documentação e Finalização

- [ ] 2.1 Verificar que todos os READMEs (`README.md`, `README.pt-BR.md`, `README.es.md`, `README.id.md`, `README.ru.md`) referenciam corretamente as imagens atualizadas e exibem paridade
- [ ] 2.2 Rodar `cargo test --workspace && cargo clippy --workspace --all-targets` e `cargo fmt --all --check` para garantir integridade do repositório
- [ ] 2.3 Revisar a tela Sobre (Configurações do Lume › Sobre o Lume) com `cargo run -p lume-desktop -- --screenshot about a.png`; sem componentes novos ou alterações de licença/repositório/versão, registrar "Sobre revisado, sem mudança"

## 3. Desativar também as recomendações

- [ ] 3.1 Nos 5 READMEs, passo 6 da Opção 2: o último recurso desativa `com.google.android.tvlauncher` e `com.google.android.tvrecommendations`, com uma frase sobre o congelamento do Lume que só o launcher desativado causa
- [ ] 3.2 Nos 5 READMEs, "Voltar ao launcher original": `pm enable` dos dois pacotes antes de `adb uninstall app.lume.tv`
- [ ] 3.3 Nos 5 READMEs, prompt da Opção 1: novo passo, se o Home ainda abrir o launcher original, pedir confirmação e desativar os dois pacotes juntos, explicando como reverter; manter a regra de não rodar outros comandos sem pedir
- [ ] 3.4 Conferir a paridade dos comandos entre os 5 idiomas (`grep -c tvrecommendations README*.md` igual em todos)

## Workflow follow-up

- Arquivar a change com `openspec archive atualize-prints-readme`
- Merge na branch `main` com squash conforme fluxo do projeto

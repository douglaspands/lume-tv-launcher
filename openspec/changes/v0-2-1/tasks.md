# Tasks

## 1. Ponte Java e manifesto

- [ ] 1.1 Em `AndroidManifest.xml`, declarar `com.android.tv.action.VIEW_INPUTS` em `<queries>`. Verificar com `adb shell dumpsys package app.lume.tv` e com o teste da tarefa 1.3.
- [ ] 1.2 Em `LumeBridge`, adicionar os comandos `inputs` ("1" ou "0", via `resolveActivity`) e `open-inputs` (`startActivity` com `FLAG_ACTIVITY_NEW_TASK`, "ok" ou "missing"); conferir `proguard-rules.pro`. Verificar com `scripts/container-build.sh release` e `verify-apk.py` (limite 6 MB).
- [ ] 1.3 Em `lume-app`, expor `inputs_available()` e `open_inputs()` na ponte (`bridge.rs`) e tratar `Effect::OpenInputs` em `android/mod.rs`. Verificar com `cargo test --workspace` e `cargo clippy --workspace --all-targets` limpos.

## 2. Barra superior e configurações (Rust)

- [ ] 2.1 Adicionar o ícone de entradas em `scripts/subset-fonts.py` e regerar `icons.rs` e a fonte; comparar com o ícone do Google TV numa captura do desktop. Verificar com `cargo run -p lume-desktop -- --screenshot home h.png`.
- [ ] 2.2 Em `topbar.rs`, incluir o botão de entradas (novo `ids::INPUTS`) à esquerda da engrenagem, presente só quando a ponte diz que existe; foco e contorno azul como a engrenagem. Teste de foco com e sem o botão em `app/tests.rs`.
- [ ] 2.3 Em `mod.rs`, a engrenagem devolve `Effect::OpenAndroidSettings` e o botão devolve `Effect::OpenInputs`; `open_settings()` só pelo tile `GRID_SETTINGS`. Testes: OK na engrenagem, OK nas entradas, tile abre as configurações do Lume.
- [ ] 2.4 Remover `settings_return` e `ITEM_ANDROID`; voltar das configurações leva o foco ao `GRID_SETTINGS` na aba Apps. Reescrever os testes que assumiam engrenagem → configurações do Lume (`tests.rs` ~419-488, 932, 968, 1790). Verificar com `cargo test --workspace`.

## 3. Textos e grafia "Lume"

- [ ] 3.1 Em `values/strings.xml` e `values-pt-rBR/strings.xml`: `app_name` "Lume", "About Lume", "Sobre o Lume", comentários, e remover `settings_android`. Verificar com `cargo test -p lume-strings` e um `grep -rn "\blume\b"` sem sobras em texto de usuário.
- [ ] 3.2 Atualizar README, `docs/translations.md`, `docs/device-notes.md`, `CLAUDE.md` e a prosa das specs tocadas para "Lume" e para o novo comportamento da engrenagem. Verificar por `grep` e `openspec validate v0-2-1 --strict`.

## 4. Painel de estatísticas

- [ ] 4.1 Confirmar na TCL e na AIWA o que um app lê: `/proc/meminfo`, `/proc/stat`, frequência dos núcleos e `StatFs`; registrar em `docs/device-notes.md`. Verificar com um APK de debug e `adb logcat`.
- [ ] 4.2 Na ponte Java, adicionar o comando `stats` (memória e disco); no Rust, interpretar a resposta e calcular o uso de CPU por duas leituras. Verificar com testes unitários do interpretador (resposta completa, sem CPU, malformada) e `cargo test --workspace`.
- [ ] 4.3 Desenhar o painel à direita da tela de Configurações (cartão, barras na cor primária, traço quando falta CPU), sem foco. Verificar com `lume-desktop --screenshot settings` em 1280x720 e 1920x1080.
- [ ] 4.4 Ligar o temporizador de 2 s só com `Screen::Settings` ativa e cancelá-lo ao sair, abrir a Sobre ou o editor e ir a segundo plano; respeitar `Shell::is_busy`. Verificar com testes do laço (nenhum quadro pedido fora da tela) e `cargo clippy --workspace --all-targets`.

## 5. Versão e verificação em dispositivo

- [ ] 5.1 Subir a versão para 0.2.1 (`Cargo.toml`, `CLAUDE.md`, nomes de APK, `verify-apk.py`); gerar `dist/lume-0.2.1-release-debugsigned.apk` e verificar com `verify-apk.py` (limite 6 MB).
- [ ] 5.2 Instalar na TCL (`.6`) e na AIWA (`.3`): botão de entradas abre o menu, engrenagem abre as configurações do Android, tile "Lume" abre as do Lume, Voltar devolve o foco ao tile. Confirmar também o painel de estatísticas com valores coerentes. Registrar em `docs/device-notes.md`.
- [ ] 5.3 Android 9: validar num emulador Android TV 9 (API 28) a instalação, a engrenagem e o botão de entradas escondido quando a ação não resolve; registrar o resultado ou, sem emulador, declarar a lacuna. Medir PSS, quadros perdidos (com a tela de Configurações aberta) e retorno a quente na AIWA (`low_ram`) contra os orçamentos de `resource-efficiency`.

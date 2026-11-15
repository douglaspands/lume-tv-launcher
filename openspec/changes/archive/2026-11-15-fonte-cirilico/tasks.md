# Tasks

## 1. Fonte

- [x] 1.1 `scripts/subset-fonts.py`: baixar a Manrope variável do `google/fonts` (commit fixado), instanciar 500/700/800, fazer o subset do cirílico (U+0400–045F, U+0490–0491, U+2116) com kerning e gravar `Manrope-Cyrillic-<peso>.ttf` e `OFL-Manrope.txt`; conferir que a Plus Jakarta e a Lucide regeneradas não mudam — 11,5 KB por peso (34,7 KB no total); Plus Jakarta e Lucide regeneradas só mudaram no carimbo de data e foram mantidas como estavam
- [x] 1.2 `fonts.rs`: constantes das faces de cirílico, `CYRILLIC_FACES` ao lado de `TEXT_FACES`, testes de cobertura do russo e do ucraniano em cada peso e orçamento de tamanho atualizado

## 2. Motor de texto

- [x] 2.1 `TextEngine`: carregar a companheira de cirílico de cada face; `FontId::Script`; ordem de busca ícones → Plus Jakarta → Manrope → sistema → glifo ausente
- [x] 2.2 Testes: cirílico vem da Manrope no peso pedido mesmo com `SystemFonts`; latim continua na Plus Jakarta; kerning só dentro da mesma fonte; métricas verticais inalteradas — teste de reserva do sistema ajustado: o cirílico agora vem da Manrope (altura do "Я" de ~15,8 px contra 13,2 px da caixa sintética)
- [x] 2.3 Desktop: captura em russo sem `LUME_FALLBACK_FONT` desenha cirílico; atualizar o comentário de `tools/lume-desktop/src/screenshot.rs` (a frase dos READMEs sobre `LUME_FALLBACK_FONT` descreve como os prints atuais foram feitos e muda na change `atualize-prints-readme`, ao refazê-los) — `cargo run -p lume-desktop -- --locale ru --screenshot settings` sem a variável desenha o cirílico

## 3. Validação no emulador (`tv_api28`) e nas TVs (TCL, Android 11; AIWA, Android 14)

- [x] 3.1 Build debug, instalar no emulador, idioma do sistema em russo; capturar Configurações ("О Lume"), Ferramentas › Imagem ("Overscan и геометрия") e a grade de Apps; comparar com o antes — antes: cirílico na Roboto Regular ("О Lume", "Все приложения", título "Настройки" fino); depois: Manrope no peso do latim nas três telas
- [x] 3.2 Build release, instalar na TCL e na AIWA (o autor autorizou, 2026-11-15) com o idioma em russo; capturar as mesmas telas; na AIWA conferir que o peso do cirílico segue o do latim (Roboto variável no Android 14) — TCL (Android 11, idioma do sistema) e AIWA (Android 14, idioma só do Lume com `cmd locale set-app-locales`): cirílico no peso do latim nas duas
- [x] 3.3 Memória (`dumpsys meminfo app.lume.tv`, `Private Dirty` sem `GL`/`EGL mtrack`) em primeiro plano, dentro de 32 MB em cada TV; tamanho do APK com `verify-apk.py` — `Private Dirty` sem `mtrack`: TCL 14,4 MB, AIWA 14,0 MB; APK release 23,26 MB (+68 KB), `verify-apk.py` sem erro
- [x] 3.4 Voltar o idioma do emulador e das TVs ao original (TCL e AIWA em `pt-BR`) e o Lume como Home — emulador en-US, TCL pt-BR, AIWA sem idioma próprio do Lume; Lume como Home nas duas TVs

## 4. Fechamento

- [x] 4.1 `cargo test --workspace && cargo clippy --workspace --all-targets` limpos e `cargo fmt --all` — 791 testes, clippy e fmt limpos
- [x] 4.2 `openspec validate fonte-cirilico --strict`
- [x] 4.3 Revisar a tela Sobre (Configurações do Lume › Sobre o Lume): incluir "Manrope (font): SIL Open Font License 1.1" em `licenses.rs`; verificar com `cargo run -p lume-desktop -- --screenshot about a.png` — Manrope incluída na lista de licenças (captura `about` em russo conferida)

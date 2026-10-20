# lume: guia para quem retoma o projeto

lume é uma tela inicial (launcher) rápida e leve para Android TV, escrita em Rust (interface, tema, renderização OpenGL ES 2.0)
com uma camada fina de Java (`LumeActivity` e `LumeBridge`, só onde o Android exige). Orçamentos e decisões ficam nas specs.

## Onde está a verdade

- `openspec/specs/`: o comportamento atual, por capacidade (home-screen, apps-screen, launcher-shell, wallpaper, theming, resource-efficiency...).
- `openspec/changes/archive/`: o histórico das changes (`2026-10-19-lume-launcher-v0-1-0`, `2026-10-20-dock-aurora-redesign`). Nenhuma change está ativa.
- `docs/handoff-dock-aurora.md`: decisões do v0.2.0, armadilhas já resolvidas, como rodar e testar.
- `docs/device-notes.md`: medidas na TV de referência (memória, quadros, CPU, tempos).
- `docs/theme-format.md`, `docs/wallpaper.md`, `docs/translations.md`: formato do tema, papel de parede e traduções.

## Estado

v0.2.0 concluída e mesclada na `main` por squash (2026-10-20). Não há remoto git configurado; nada foi publicado.
Pendências conhecidas, que dependem do autor: keystore de release, CI e release no GitHub, tag `v0.1.0`/`v0.2.0`,
verificação em emulador Android TV e decisão sobre o orçamento do retorno a quente (a spec diz 300 ms; o sistema da TCL
gasta cerca de 400 ms só na troca de janelas, o lume 75 a 110 ms; a proposta é medir só a parte do lume, até 150 ms).

## Como trabalhar aqui

- **Uma branch por change OpenSpec**: logo após `openspec new change "<nome>"`, `git checkout -b feat/<nome>`.
- **Não commitar sem o autor pedir.** Quando pedir, commits por tema; o fechamento é `/opsx:archive` e merge **squash** na `main`.
- Fluxo OpenSpec: `/opsx:propose`, `/opsx:apply`, `/opsx:archive`. Validar com `openspec validate <change> --strict`.
- Idioma: respostas, specs e commits do projeto em português do Brasil; código, comentários e mensagens de commit em inglês.
- Neste ambiente use `/usr/bin/python3` (o `python3` do asdf pode falhar).

## Comandos

```
cargo test --workspace && cargo clippy --workspace --all-targets   # deve ficar limpo; cargo fmt --all
cargo run -p lume-desktop -- --size 1920x1080 --locale en --keys right --screenshot home h.png   # telas: home apps settings about editor
scripts/container-build.sh release       # APK em dist/ (runtime: wslc, docker, podman, nesta ordem; LUME_CONTAINER força outro)
/usr/bin/python3 scripts/verify-apk.py dist/lume-0.2.0-release-debugsigned.apk   # limite 6 MB
```

## TV de referência (testes por adb)

TCL 55C715 (2 GB de RAM, 4 núcleos), `adb -s 192.168.0.6:5555`; o autor autorizou testar nela.

- Instalar: `adb uninstall app.lume.tv` antes (a chave de debug muda a cada build no container), depois `adb install dist/<apk>` e
  `adb shell cmd package set-home-activity app.lume.tv/.LumeActivity`.
- Capturas: `adb exec-out screencap -p > x.png`. Memória: `dumpsys meminfo app.lume.tv`.
- Quadros perdidos: `dumpsys SurfaceFlinger --latency "<camada>"` (o nome da camada, `app.lume.tv/app.lume.tv.LumeActivity#N`, muda
  depois de reinstalar: procure com `--list`); conte intervalos de 33 ms ou mais.
- Abrir outro app para testar segundo plano: `adb shell monkey -p <pacote> -c android.intent.category.LEANBACK_LAUNCHER 1`.

## Decisões fixas do autor

- Papel de parede: aurora gerada pelo projeto (`tools/wallpaper/aurora06.py`); nenhuma foto de terceiros.
- Tela Início: só o dock (até 12 favoritos, Magnification). Sem carrossel nem seções.
- Abas: mover o foco para uma aba troca a tela sem OK, com fade de 180 ms. Sublinhado azul com o foco na barra, branco com o foco no conteúdo.
- Cores por papel (`--primary` azul `#4DB3FF`, `--secondary`); todo contorno de foco é azul primário; o nome do app em foco fica branco.
- Grade de Apps: 6 colunas, ordem alfabética pelo nome mostrado (muda com o idioma do sistema). O tile "Lume" é um app como os outros
  (retangular, logo) e abre as Configurações do lume. A marca se escreve "Lume" na interface.
- Home com o lume visível leva à aba Início; voltando de um app aberto pelo lume, restaura tela, foco e rolagem (uma vez).

## Armadilhas já resolvidas

- **Nunca desenhar camada translúcida de tela cheia**: na GPU da TV ela custou 54% dos quadros da grade. O escurecimento das telas de
  texto é feito no próprio desenho do papel de parede (`Painter::wallpaper(true)`, sobre o fundo `--bg` em que o quadro é limpo).
- Animação com valor de repouso 0 é descartada pela limpeza (`Animations::prune`); passe o valor inicial em `animate_to`.
- Métodos Java chamados só pelo Rust (JNI) precisam de `-keep` em `android/app/proguard-rules.pro`.
- Carregar ícones durante uma animação trava o dock: o laço só carrega quando `Shell::is_busy` é falso.
- `entry_down`/`entry_up` de `Row` são do destino do foco, não da origem.
- O Android da TCL encerra o processo do lume em segundo plano quando um app pesado abre; o retorno é então uma abertura a frio.

## Orçamentos (spec `resource-efficiency`)

PSS até 45 MB em primeiro plano e 20 MB em segundo; APK até 6 MB; no máximo 1% de quadros de 33 ms ou mais na navegação; retorno a quente
até 300 ms; o laço não pede quadros parado além do relógio. Última medida (v0.2.0): 17 a 20 MB e 12,2 MB de PSS, APK de 3,15 MB, 0 a 1% de
quadros perdidos, abertura a frio de 373 ms.

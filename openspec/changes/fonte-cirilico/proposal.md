## Why

Em idiomas de escrita não latina, como o russo, a interface mistura duas fontes na mesma linha. A Plus Jakarta Sans embutida só cobre o latim (Latin-1 e Latin Extended-A, `crates/lume-core/src/fonts.rs`), então o cirílico vem da fonte do sistema (`text/system_fonts.rs`), enquanto as palavras em latim do mesmo texto continuam na Plus Jakarta. Exemplos vistos no emulador `tv_api28` em russo (2026-11-15, change `ferramentas`):

- "О Lume" nas Configurações: "О" na fonte do sistema e "Lume" na Plus Jakarta, mais grossa e com outro desenho.
- "Overscan и геометрия" (Ferramentas › Imagem) e o cabeçalho "OVERSCAN И ГЕОМЕТРИЯ": "Overscan" se destaca do resto.
- O mesmo vale para "Wi-Fi", "Mbps", "GHz", "KEYCODE_*" e nomes de apps em latim entre textos em cirílico.

O resultado parece um erro de digitação ou de ênfase, quebra a coesão tipográfica que o design system pede ("Tipografia": uma família, três pesos) e afeta qualquer idioma cuja escrita a fonte embutida não cobre (cirílico, grego e outros).

## What Changes

- **Decisão (2026-11-15, autor)**: embutir a Manrope (SIL OFL 1.1), só os glifos cirílicos, nos pesos 500, 700 e 800, como reserva da Plus Jakarta Sans. O latim continua na Plus Jakarta; o cirílico vem da Manrope no mesmo peso. As duas têm desenho geométrico e altura de x quase iguais (0,54 contra 0,544 do corpo), e a mistura deixa de ser visível. Alternativas descartadas no design: a fonte do sistema (no Android 12 ou mais novo a Roboto é variável e o `fontdue` só desenha o peso 400; a AIWA tem Android 14) e a linha inteira em Manrope (custo três vezes maior e mais código na medição).
- A fonte do sistema continua como reserva para o que nenhuma fonte embutida cobre (grego, chinês e outras escritas em nomes de apps).
- O desktop passa a desenhar russo sem `LUME_FALLBACK_FONT`, e as capturas de verificação em russo vêm do emulador `tv_api28`.
- A tela Sobre passa a listar a Manrope.

## Capabilities

### New Capabilities
<!-- Nenhuma. -->

### Modified Capabilities
- `localization`: o requisito de exibir caracteres fora da fonte embutida passa a dizer como a linha é composta (sem misturar famílias por palavra) e com que peso.
- `design-system`: o requisito de tipografia passa a prever a fonte usada nas escritas que a Plus Jakarta Sans não cobre.

## Impact

- **Rust (`lume-core`)**: `text/` (motor de texto, `fallback.rs`, `system_fonts.rs`), possivelmente `fonts.rs` e `scripts/subset-fonts.py` se uma família nova for embutida.
- **Desktop (`lume-desktop`)**: fonte de reserva para as capturas em russo.
- **APK**: só cresce se uma fonte for embutida; medir com `verify-apk.py`.
- **Tela Sobre**: nova licença de fonte, se uma família for embutida.
- Sem Java.

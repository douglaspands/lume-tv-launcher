## Why

Em idiomas de escrita não latina, como o russo, a interface mistura duas fontes na mesma linha. A Plus Jakarta Sans embutida só cobre o latim (Latin-1 e Latin Extended-A, `crates/lume-core/src/fonts.rs`), então o cirílico vem da fonte do sistema (`text/system_fonts.rs`), enquanto as palavras em latim do mesmo texto continuam na Plus Jakarta. Exemplos vistos no emulador `tv_api28` em russo (2026-11-15, change `ferramentas`):

- "О Lume" nas Configurações: "О" na fonte do sistema e "Lume" na Plus Jakarta, mais grossa e com outro desenho.
- "Overscan и геометрия" (Ferramentas › Imagem) e o cabeçalho "OVERSCAN И ГЕОМЕТРИЯ": "Overscan" se destaca do resto.
- O mesmo vale para "Wi-Fi", "Mbps", "GHz", "KEYCODE_*" e nomes de apps em latim entre textos em cirílico.

O resultado parece um erro de digitação ou de ênfase, quebra a coesão tipográfica que o design system pede ("Tipografia": uma família, três pesos) e afeta qualquer idioma cuja escrita a fonte embutida não cobre (cirílico, grego e outros).

## What Changes

- Definir a regra tipográfica para escritas fora da fonte embutida: uma linha de texto não mistura famílias por palavra. Opções a avaliar no design:
  - **Fonte do sistema para a linha inteira**: quando o texto tem algum caractere fora da Plus Jakarta, a linha toda usa a fonte do sistema, no peso pedido (500, 700 ou 800, o mais próximo que o sistema tiver).
  - **Fonte do sistema para o idioma**: com o idioma da interface numa escrita não coberta, todos os textos usam a fonte do sistema; nomes de apps continuam por linha.
  - **Embutir uma família com cirílico** parecida com a Plus Jakarta, só os glifos cirílicos (subset), medindo o custo no APK (limite de 40 MB).
- Fazer a fonte de reserva respeitar o peso pedido. Hoje o cirílico pode sair num peso diferente do latim ao lado.
- Capturas de verificação em russo (emulador e desktop; o desktop hoje não tem fonte de reserva com cirílico e desenha caixas vazias, o que também precisa de solução para as capturas).

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

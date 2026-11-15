# Design

## Context

A Plus Jakarta Sans embutida (`crates/lume-core/assets/fonts/PlusJakartaSans-*.ttf`) cobre só o latim, e a fonte original também não tem nenhum glifo cirílico (conferido no commit `18d1cd2` do repositório da tokotype). Hoje o `TextEngine::resolve` procura cada caractere na face do peso pedido e, se ela não tem o caractere, recorre ao `FallbackFonts` (no Android, `SystemFonts`), que só carrega as fontes de peso 400 do `fonts.xml`. Resultado em russo: o cirílico sai em Roboto Regular ao lado do latim em Plus Jakarta 700 ou 800, com outro desenho e outro peso ("О Lume", "Overscan и геометрия").

## Goals / Non-Goals

**Goals:**
- O cirílico combina com o latim ao lado: mesma família visual e mesmo peso (500, 700 ou 800).
- O resultado é igual em qualquer TV (Android 9, 11 e 14) e no desktop, sem depender das fontes do sistema.
- Custo pequeno no APK e na memória.

**Non-Goals:**
- Grego, chinês, árabe e outras escritas: continuam pela fonte do sistema, letra a letra (só aparecem em nomes de apps).
- Shaping, escrita da direita para a esquerda.
- Mudar a escala tipográfica ou os tokens.

## Decisions

- **Manrope como reserva embutida do cirílico** (escolha do autor, 2026-11-15, entre três opções comparadas numa imagem). Manrope (SIL OFL 1.1, Google Fonts) tem desenho geométrico próximo da Plus Jakarta: altura de x 0,540 contra 0,544, altura das maiúsculas 0,720 contra 0,745. Sem reescala: a diferença de 3% nas maiúsculas não se nota.
  - Alternativa: fonte do sistema por linha. Descartada: no Android 12 ou mais novo a Roboto é uma fonte variável e o `fontdue` desenha só a instância padrão (400), então o peso não seria respeitado na AIWA (Android 14); o desktop seguiria sem cirílico.
  - Alternativa: linha inteira em Manrope (latim incluído). Descartada: ~51 KB por peso contra ~15 KB, e a decisão por linha mexe em medição, reticências e cache.
- **Origem**: `ofl/manrope/Manrope[wght].ttf` do repositório `google/fonts`, fixado no commit `b31870aff700ab7a1d74fa0c6887d95beb9e0037`. O `scripts/subset-fonts.py` instancia o eixo `wght` em 500, 700 e 800 (`fontTools.varLib.instancer`) e faz o subset para U+0400–045F, U+0490–0491 e U+2116 (№), com o kerning convertido para a tabela `kern`, como já faz com a Plus Jakarta. Saída: `crates/lume-core/assets/fonts/Manrope-Cyrillic-<peso>.ttf` e `OFL-Manrope.txt`.
- **Ordem de busca de um caractere**: ícones (área de uso privado) → Plus Jakarta no peso pedido → Manrope no mesmo peso → fontes do sistema → glifo de "ausente" da Plus Jakarta. No `TextEngine`, cada face de texto passa a ter uma companheira de cirílico, e o `FontId` ganha a variante `Script(u8)` (índice da face). O kerning já é aplicado só entre glifos da mesma fonte, então pares cirílicos têm kerning da Manrope e a fronteira latim/cirílico não tem kerning.
- **Métricas verticais** (ascendente, descendente, altura das maiúsculas) continuam vindo da Plus Jakarta, para a régua não mudar.
- **Desktop**: com o cirílico embutido, `LUME_FALLBACK_FONT` deixa de ser necessária para russo; ela continua aceita para outras escritas.

## Risks / Trade-offs

- **[Orçamento de fontes]** O teste `total_font_size_stays_under_the_budget` limita as fontes a 200 KB; hoje são ~102 KB e a Manrope soma ~45 KB. → Medir com o teste; o APK (22,1 MB) fica longe do limite de 40 MB.
- **[Memória]** Três faces a mais carregadas pelo `fontdue` no início. → São carregadas como as outras faces embutidas; medir o `Private Dirty` no emulador, que deve continuar bem abaixo de 32 MB.
- **[Diferença sutil de desenho]** Algumas letras podem destoar ("О" redondo da Manrope ao lado do "O" da Plus Jakarta). → Conferência visual no emulador em russo, nas telas citadas na proposta.

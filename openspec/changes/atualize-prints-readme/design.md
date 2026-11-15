# Design

## Context

O Lume documenta suas telas através de quatro imagens por idioma: `home-<lang>.jpg`, `apps-<lang>.jpg`, `settings-<lang>.jpg` e `editor-<lang>.jpg`, em `docs/screenshots/`. Os 5 idiomas suportados são `en`, `pt-BR`, `es`, `id` e `ru`.
O runner `tools/lume-desktop` suporta a geração headless de screenshots via flag `--screenshot <SCREEN> <FILE>` e `--locale <TAG>`.

## Goals / Non-Goals

**Goals:**
- Gerar todas as 20 capturas de tela (4 telas × 5 idiomas) na resolução padrão (1920x1080) usando o `lume-desktop`.
- Garantir que a geração respeite a estética pretendida (sem banners sintéticos das fontes demo, usando `LUME_NO_SYNTHETIC_ICONS=1`, e suporte à fonte de fallback para russo se necessário).
- Converter/comprimir os arquivos gerados para JPEG com qualidade balanceada (peso em torno de 70-130 KB por imagem), substituindo as imagens existentes em `docs/screenshots/`.
- Garantir que todos os 5 READMEs apontem corretamente para os arquivos atualizados.

**Non-Goals:**
- Alterar layout ou mecânica das telas do Lume.
- Modificar o código nativo ou rust da aplicação além do que for necessário para ferramentas de captura.
- Mudar nomes ou estrutura dos links existentes nos READMEs.

## Decisions

- **Ferramenta de geração**: Usar `cargo run -p lume-desktop -- --size 1920x1080 --locale <locale> --screenshot <screen> <output.png>`.
- **Formato dos arquivos**: JPEG (usando compressão via Python com PIL/Pillow com qualidade 85-90) para manter o repositório leve (Git repo sem inchaço desnecessário por PNGs de 400KB+).
- **Consistência visual**: Executar com `LUME_NO_SYNTHETIC_ICONS=1` para que os blocos de ícones exibam suas letras e cores sem textura artificial, condizente com a versão atual da documentação.

- **Launcher original e recomendações juntos**: os READMEs desativam `com.google.android.tvlauncher` e `com.google.android.tvrecommendations` no mesmo passo (`pm disable-user --user 0`), e reativam os dois com `pm enable`. O prompt do assistente só faz isso com a confirmação do usuário, como hoje. Os dois pacotes só existem no Android TV do Google; em TVs sem eles, o comando falha sem efeito, e o README diz para ignorar esse erro.

## Risks / Trade-offs

- **[Renderização de fontes no locale russo (`ru`)]** → Verificar se o ambiente possui fonte cirílica instalada no sistema para o `LUME_FALLBACK_FONT`. Se necessário, apontar para fonte padrão TTF do sistema (ex.: DejaVu Sans / Roboto).
- **[Tamanho do commit/repositório]** → Garantir compressão JPEG para evitar introdução de dezenas de megabytes no git.

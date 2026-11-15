# Proposal

## Why

As capturas de tela exibidas nos READMEs (`README.md`, `README.pt-BR.md`, `README.es.md`, `README.id.md`, `README.ru.md`) precisam ser atualizadas com as versões mais recentes das telas do Lume em todas as cinco línguas suportadas (Início/Dock, Apps, Configurações e Editor de favoritos). Manter os prints fiéis ao design atual (ícones sem artefatos sintéticos, renderização correta de fontes e suporte a Full HD) melhora a apresentação visual do projeto para novos usuários e desenvolvedores.

## What Changes

- Regeneração das capturas das 4 telas principais (`home`, `apps`, `settings`, `editor`) para os 5 idiomas suportados (`en`, `pt-BR`, `es`, `id`, `ru`) em `docs/screenshots/`.
- Uso do runner `lume-desktop` com as variáveis de ambiente adequadas (`LUME_NO_SYNTHETIC_ICONS=1` e fonte de fallback para cirílico via `LUME_FALLBACK_FONT` se aplicável).
- Otimização e compressão das capturas para formato JPEG de alta qualidade e tamanho compacto, mantendo o repositório leve.
- Verificação das referências em cada um dos cinco arquivos `README.*.md` garantindo exibição correta e paridade.

- **Desativar também as recomendações**: no último recurso do passo 6 da instalação manual (desativar o launcher original), e no prompt do assistente de IA, desativar junto o serviço `com.google.android.tvrecommendations`. Desativando só o `com.google.android.tvlauncher`, esse serviço fica tentando reativar o launcher do Android e congela o Lume (relato do autor na TCL, 2026-11-15; lá os dois estão desativados). "Voltar ao launcher original" reativa os dois, nos 5 READMEs.

## Capabilities

### New Capabilities

*(Nenhuma nova capacidade introduzida; a change é puramente de documentação e ativos visuais)*

### Modified Capabilities

- `installation-docs`: o último recurso da instalação manual e o prompt do assistente de IA passam a desativar, junto com o launcher original, o serviço de recomendações do Android TV, e a reversão reativa os dois.

## Impact

- Ativos em `docs/screenshots/*.jpg`.
- Documentação nos 5 arquivos `README.md`, `README.pt-BR.md`, `README.es.md`, `README.id.md` e `README.ru.md`.
- Nenhuma alteração nas crates de produção (`lume-core`, `lume-app`) ou no APK gerado.

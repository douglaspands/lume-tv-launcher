# Proposta: papel de parede pelo celular

## Why

Hoje o Lume só oferece os 6 papéis de parede do catálogo, e a TV não tem um jeito confortável de receber uma imagem do
usuário: digitar no controle é inviável e a TV não tem galeria de fotos. O celular, que está na mão de quem usa a TV,
já tem as fotos, uma câmera que lê QR e um navegador que decodifica qualquer formato. Esta change usa isso para
trazer uma imagem própria até a TV, sem app no celular e sem Java novo.

## What Changes

- No modo edição (Configurações › Tela Início), a fila de papéis de parede ganha um mini-card, o primeiro da fila, que é
  um slot único: "+ Do celular" enquanto vazio, "Minha imagem" depois do primeiro envio aplicado. Na primeira abertura
  o foco fica na Aurora, o papel padrão.
- Tela **Receber imagem** na TV: código QR, endereço para digitar, código curto, passos e estado ("Celular conectado ·
  editando…" enquanto alguém usa a página). Abrir a tela liga um servidor HTTP mínimo na rede local.
- **A edição acontece no celular.** A página servida pela TV, com o logo e o nome "Lume" e no idioma do aparelho, é
  responsiva em smartphone, tablet e computador. Ela decodifica a foto no próprio navegador (JPEG, PNG, WebP e HEIC), corrige a rotação e abre uma prévia 16:9 interativa com cinco modos: Ajustar, Preencher (zoom de 100 a 200% por
  pinça, botões ou roda do mouse, e arrastar para mover), Esticar, Fundo desfocado e Repetir (a foto inteira no centro e
  cópias lado a lado ou espelhadas em volta, escala de 25 a 100%). Ao enviar, o celular compõe a imagem final em 1920x1080
  e a manda em QOI, o formato que a TV já lê. A TV revalida tudo.
- **Gordura de limite**, na página: uma foto quase 16:9 (±5%) começa em Preencher, sem faixas. Fora disso começa em
  Ajustar, com faixas pretas nas laterais (vertical) ou em cima e embaixo (panorâmica). Com o lado menor entre 720 e
  1079 px, a foto é aceita com aviso. De 64 a 719 px, só o modo Repetir fica disponível. Abaixo de 64 px, é recusada.
- Tela **Confirmar** na TV: o resultado em tela cheia, como seria o papel de parede, com "Cancelar" e "Aplicar". Enquanto
  ela está aberta o servidor segue ligado: o usuário pode ajustar de novo no celular e reenviar, e a TV troca a imagem.
  Aplicar grava o resultado e desliga o servidor.
- **HEIC em qualquer navegador**: onde o navegador não abre HEIC (Chrome do Android e do computador), a página
  carrega um decodificador em WebAssembly (libheif com libde265, LGPL-3.0), servido pela TV a partir dos assets do APK e
  baixado só quando preciso. As fotos dos aparelhos da casa são HEIC.
- **Prazo que não expira à toa**: a página avisa a TV a cada 30 s enquanto está visível, com o tempo desde o último toque;
  a TV distingue edição de inatividade. Os 5 minutos contam só sem edição, o relógio aparece só no último minuto, e ao
  expirar sai um código novo sozinho (até 3 vezes) antes do "Tempo esgotado".
- O resultado aplicado fica em `files/` (o sistema não apaga) e vira o mesmo cache RGB de 1920x1080 dos papéis do
  catálogo. O desenho, o escurecimento das telas de texto e a memória gráfica não mudam. A TV não guarda a foto original.
- Menu de contexto no card "Minha imagem" (botão Menu, ou OK quando ela já é a ativa): Usar esta imagem, Trocar imagem
  e Remover.
- **BREAKING (regra de plataforma)**: a permissão `INTERNET` deixa de ser só para listar interfaces e passa a servir
  para escutar na rede local enquanto as telas Receber imagem e Confirmar estão abertas. O Lume continua sem abrir
  conexões de saída.

## Capabilities

### New Capabilities

- `phone-companion`: servidor local temporário, código de acesso, QR e endereço, prazo com edição distinta de
  inatividade, a página do Lume para o celular e o recebimento e a validação do envio. Hoje só tem o papel de parede;
  no futuro pode receber outras ferramentas.
- `custom-wallpaper`: o slot "Minha imagem", a edição no celular (tolerâncias, modos, zoom, posição e repetição), a
  confirmação na TV e a persistência do resultado.

### Modified Capabilities

- `wallpaper`: o papel de parede ativo pode ser o do catálogo ou "Minha imagem", e o cache ao acordar passa a cobrir
  a imagem própria, decodificada de novo do arquivo guardado se o sistema apagar o cache.
- `home-editor`: a seleção de papel de parede ganha o card do celular (o primeiro), e a chave `wallpaper` de `config.v1`
  aceita o valor da imagem própria, com a chave `custom_rev`.
- `platform-compatibility`: a regra da permissão `INTERNET` passa a permitir o servidor local temporário.
- `launcher-shell`: o botão Menu passa a abrir o menu de contexto do card "Minha imagem" no modo edição.
- `design-system`: medidas das telas Receber imagem e Confirmar, do código QR e da edição na página.

## Impact

- **Código Rust**: `crates/lume-core` ganha o servidor (std, uma thread), o QR, as telas Receber imagem e Confirmar e o
  card do editor. Em `wallpaper/` entra a fonte "Minha imagem" (o resultado guardado) e o cache. Em `crates/lume-app`,
  o IPv4 vem do `SIOCGIFCONF` que já existe em `device_info.rs`. Não entra Java.
- **Página**: os modos de enquadramento, os gestos e a composição em 1920x1080 ficam no JavaScript da página (canvas),
  embutida no APK, de poucas dezenas de KB.
- **Dependência nova**: `qrcodegen` (MIT, sem dependências), que vai para a tela Sobre e o `deny.toml` já permite.
  Não entra decodificador de JPEG, PNG ou HEIC na TV.
- **Decodificador HEIC da página**: `heic.js` e `heic.wasm` (libheif e libde265, LGPL-3.0, ~1,5 MB), gerados por um script
  a partir dos fontes com versão e hash fixos (Emscripten num contêiner) e commitados; o APK os leva uma vez em
  `assets/companion/`. Entram na tela Sobre e no `docs/`. O HEVC tem patentes: incluir o decodificador é decisão do autor
  (2026-11-15).
- **Recursos**: o APK cresce ~1,5 MB (dentro dos 40 MB). Na TV, só a imagem final (6,2 MB em RGB) fica em memória, durante a confirmação; a foto original nunca
  chega à TV. O PSS continua dentro dos 64 MB, e ao sair a memória volta. No celular, a cópia de trabalho de até 4 MP
  fica no navegador.
- **Traduções**: textos novos da TV e da página em `values*/strings.xml` (en, pt-BR, es, id, ru).
- **Segurança**: só HTTP na rede local, com código de acesso, prazo de inatividade, limite de corpo (9 MB), poucas
  tentativas e servidor desligado ao aplicar ou sair.

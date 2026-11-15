# Design: papel de parede pelo celular

## Context

O motivo está em proposal.md. Do que existe hoje, isto molda a solução:

- **Papel de parede** (`crates/lume-core/src/wallpaper/mod.rs`): o catálogo é `WallpaperId` com dados `'static`
  (`Wallpaper { asset, width, height, stars }`). `load_in_background(id)` decodifica o QOI do APK numa thread, guarda
  o RGB cru de 1920x1080 em `cache/` e avisa o laço (`set_ready_notifier`). As miniaturas do editor vêm de um atlas
  único (`decompress_thumbnails`, `thumb_uv`).
- **Configuração**: `config.v1` em pares chave e valor (`storage.rs`). Um `wallpaper` desconhecido já vira Aurora
  (`home/config.rs`, `from_str(...).unwrap_or(Aurora)`).
- **Rede**: o IPv4 vem de `SIOCGIFCONF` em `crates/lume-app/src/android/device_info.rs`, porque `getifaddrs` é bloqueado
  no Android 11. A permissão `INTERNET` já está no manifesto.
- **Decodificadores na TV**: só `qoi` e `miniz_oxide`. Nenhum decodificador de JPEG, PNG ou HEIC.
- **Traduções**: `Translator::new(&[preferidos])` já resolve uma lista de idiomas, o que serve ao `Accept-Language`.
- **Telas**: o `enum Screen` em `app/mod.rs`, o caminho, o título e o conteúdo em y=152 de `page.rs`, os cartões e o menu
  de contexto já existentes.
- **Restrições**: tudo em Rust, sem Java novo. PSS de até 64 MB. Sem camada translúcida de tela cheia. Sem quadros
  com a tela ociosa.

## Goals / Non-Goals

**Goals:**
- A edição acontece no aparelho do usuário, com toque (pinça e arraste) ou mouse, sobre uma prévia que responde na hora,
  sem depender da CPU da TV.
- A TV recebe só o resultado em 1920x1080, mostra-o para confirmar e o guarda. O renderizador não muda: o resultado vira o
  mesmo cache RGB de 1920x1080 do catálogo.
- O servidor, o QR, as telas e a validação ficam em Rust, testáveis no `lume-desktop` com um celular real na mesma rede.
- O código não expira enquanto alguém está editando.

**Non-Goals:**
- HTTPS, contas, pareamento persistente ou acesso fora da rede local.
- Mais de uma imagem própria, galeria ou histórico.
- Reajustar pela TV sem o celular (a TV não guarda a foto original).
- Outras ferramentas na página (a estrutura comporta, mas ficam fora desta change).
- Decodificar JPEG, PNG, WebP ou HEIC na TV.

## Decisions

### 1. O navegador decodifica, edita e compõe; a TV recebe o resultado em QOI
A página decodifica com `createImageBitmap(file, {imageOrientation: "from-image"})`, que aplica o EXIF, e reduz por
área, em passos de no máximo metade, até uma cópia de trabalho de ≤ 4 MP que fica só no navegador. A edição desenha a
prévia num `<canvas>` 16:9 do tamanho da tela do aparelho a cada gesto. Ao enviar, a página compõe o resultado num canvas
de 1920x1080 com o mesmo código da prévia, codifica QOI em JS (cerca de 100 linhas) e envia. **Por quê:** a CPU fraca da
AIWA ficava compondo a prévia a cada tecla (o controle "não se mexia" e "pulava"), a foto de 4 MP em memória passava
de 12 MB na TV, e pinça e arraste são muito melhores que setas e "−/+" no controle remoto. **Alternativa:** a edição na
TV com os modos em Rust (a primeira versão desta change), descartada pelo autor depois do teste nas TVs.
Um HEIC que o navegador não abre passa pelo decodificador em WebAssembly (decisão 13) antes do mesmo caminho.

### 2. Servidor HTTP/1.1 mínimo em `std::net`, uma thread
O módulo `lume-core::companion` usa `TcpListener` no IPv4 escolhido, com `accept` bloqueante numa thread
`lume-companion`. As conexões são atendidas uma por vez, com `Connection: close`, tempo-limite de leitura e escrita
de 10 s e cabeçalhos de até 8 KB. São quatro rotas:

| Rota | Resposta |
|---|---|
| `GET /wallpaper/` | a página do papel de parede (HTML com CSS, JS e SVG embutidos), com os textos do idioma de `Accept-Language` injetados como JSON |
| `POST /wallpaper/upload?c=<código>` | corpo QOI de 1920x1080. Exige `Content-Length` (sem chunked), recusa acima de 9 MB antes de ler e responde JSON `{ok, erro}` |
| `POST /ping?c=<código>&idle=<s>` | o aviso da sessão: a página aberta e visível, com os segundos desde a última interação; responde `{ok}` |
| `GET /lib/heic.js`, `GET /lib/heic.wasm` | o decodificador HEIC (decisão 13), lido dos assets do APK; só pedido quando preciso |
| `GET /` | redireciona (302) para `/wallpaper/`, mantendo `?c=` |
| outra | 404 |

**Rotas por função.** Cada função da página tem o seu caminho (`/wallpaper/` hoje), com as suas rotas embaixo dele;
o que é da sessão (`/ping`) ou compartilhado (`/lib/`) fica fora, para servir a funções futuras sem mudar de lugar. Com
uma função só, `/` leva direto a ela; quando houver mais de uma, `/` vira a página inicial com a lista delas. O QR leva
`http://<ip>:<porta>/wallpaper/?c=<código>`, e a tela mostra para digitar `<ip>:<porta>/wallpaper`.

Para desligar, uma flag atômica é ligada e a própria TV abre uma conexão local para destravar o `accept`. Depois
disso, o listener é descartado. Não entra crate de HTTP (`tiny_http`, `hyper`), porque poucas rotas não justificam a
dependência nem a superfície. **Porta:** tenta 8642 e, se estiver ocupada, usa a porta 0 (o sistema escolhe). A
porta escolhida aparece no QR. O servidor vive enquanto a tela Receber imagem ou a tela Confirmar estiver aberta: um
envio aceito não o desliga, para o celular poder reenviar com outro ajuste; sair ou o prazo esgotado de vez o desligam.
Aplicar o desliga 10 s depois: nesse intervalo ele só responde ao `/ping`, cuja resposta leva a decisão da TV
(`{"ok":true,"tv":"pending|applied|declined"}`), e recusa envios. Depois de um envio, a página pergunta a cada 2 s; com
"applied" ela mostra o fim e tenta `window.close()` (só funciona em abas abertas por script; senão, diz que a aba pode ser
fechada).

### 3. Código de acesso de 4 caracteres
O alfabeto tem 31 símbolos, sem 0/O e 1/I/L, o que dá cerca de 923 mil combinações. Cinco erros desligam o servidor,
então a chance de acerto às cegas fica abaixo de 1 em 180 mil. O código vem de `/dev/urandom`, lido com `std::fs`, que existe
no Android e no Linux e não traz dependência nova. No QR ele vai em `?c=`, e a página o reenvia no `POST`. Digitado à mão,
a página pede o código. **Alternativa:** um token longo só no QR, que é inviável para quem digita no computador.

### 4. Escolha do IP
`lume-core` recebe do app um provedor `fn() -> Vec<(nome, Ipv4Addr)>`, preenchido pelo `SIOCGIFCONF` que já existe
no Android e por uma implementação simples no `lume-desktop`. A ordem de preferência é: faixa privada (10/8,
172.16/12, 192.168/16), depois `eth*` antes de `wlan*`, e loopback fica fora. Sem candidato, a tela mostra "sem rede"
e consulta de novo a cada 3 s enquanto está aberta. Isso é leve e não pede quadros enquanto nada muda.

### 5. QR com `qrcodegen`
É a crate de referência do Nayuki (MIT, sem dependências, ~1.000 linhas). Usa correção de erro M e versão mínima
automática. Os módulos são desenhados como retângulos na lista de desenho, sem textura, com tamanho inteiro em pixels
do buffer (`floor(320·escala / (n+8))`) e centralizados no quadrado branco. **Alternativa:** `fast_qr`, maior e com
recursos de SVG e imagem que não usamos.

### 6. Página construída na compilação
`assets/companion/index.html` é um modelo. No `build.rs` de `lume-core`, que já lê `theme.css` via `lume-css`, as
variáveis de cor e forma são injetadas e o logo vira SVG, gerado das mesmas constantes de `logo.rs` (retângulos
arredondados). Um teste compara as proporções. O HTML final fica embutido com `include_str!` e não vai para os
assets do APK. Os textos vêm de chaves novas `companion_*` em `values*/strings.xml`, e o servidor monta
`window.L = {...}` com o `Translator` do idioma do aparelho. O layout responsivo é CSS puro com três faixas
(`<600`, `600–1023`, `≥1024`), com `grid` de duas colunas no computador e área de soltar no computador
(`dragover`/`drop`). O envio usa `XMLHttpRequest`, por causa de `upload.onprogress`. A fonte é `system-ui`, para não
servir uma fonte.

### 7. Resultado guardado em `files/`, cópia no cache
Aplicar grava o resultado recebido (QOI de 1920x1080, até 9 MB) em `files/wallpaper/custom.qoi`, que o sistema não
apaga, por escrita atômica (temporário e `rename`, como `storage::save`), e uma miniatura de 128x72 ao lado. Em
`config.v1` ficam `wallpaper=custom` e `custom_rev`, um contador que sobe a cada Aplicar. O cache RGB é
`wallpaper-custom-<rev>-<versão>.rgb`, e os caches velhos são apagados. Se o sistema apagar o cache, o QOI guardado é
decodificado de novo (o mesmo caminho do catálogo, que decodifica o QOI do APK). No código, `WallpaperId` ganha
`Custom`, que usa um espaço livre do atlas de miniaturas, e o carregamento passa por `enum Source { Catalog, Custom }`.
A TV não guarda a foto original nem o enquadramento: para mudar, o usuário envia outra imagem ("Trocar imagem").

### 8. Enquadramento no JavaScript da página, verificado no Chrome
Os cinco modos ficam numa função `compose(ctx, bitmap, params, w, h)` da página, que desenha num canvas:

| Modo | Como é feito |
|---|---|
| Ajustar, Preencher, Esticar | `drawImage` com `imageSmoothingQuality = "high"` a partir da cópia de trabalho |
| Fundo desfocado | a imagem a 1/8, desfocada (`ctx.filter = "blur()"` onde existe; senão 3 passadas de box blur em JS sobre essa imagem pequena), ampliada para cobrir, e Ajustar por cima |
| Repetir | a cópia central onde Ajustar a põe; cópias em volta com `drawImage`, espelhadas com `scale(-1, 1)` na variante Espelhado |

A mesma função desenha a prévia (no tamanho da tela do aparelho) e o resultado (1920x1080), então o que se vê é o que
vai. As regras de tolerância e os casos das specs viram um teste que roda a página no Chrome sem janela pelo DevTools
(`scripts/companion-page-test.mjs`): ele escolhe fotos de teste, envia e confere o QOI recebido (faixas de 656 px na
foto vertical, cópias encostadas na Repetir, nenhuma área vazia em Preencher). O módulo Rust `wallpaper::frame` sai do
app; a TV só precisa de uma redução por área para a miniatura.

### 9. A confirmação usa a própria textura do papel de parede
A imagem recebida passa da thread do servidor ao laço por canal, já validada e decodificada em RGB, e vira o conteúdo de
`WallpaperId::Custom` enquanto a tela Confirmar está aberta (a mesma rota de envio fatiado à GPU do catálogo). Assim a TV
mostra o resultado como ele ficará, sem segunda textura. Um envio novo durante a confirmação troca a imagem. Cancelar
volta ao papel anterior; Aplicar grava e mantém. A imagem recebida (6,2 MB) é liberada ao sair da tela.

### 10. Fluxo das telas
```
Editor --OK "+ Do celular" / Menu > Trocar--> ReceiveImage --envio aceito--> ConfirmImage --Aplicar--> Editor
                                                  ^   |                         |   ^
                                                  |   +--Voltar/Cancelar--> Editor  |  (novo envio troca a imagem)
                                                  +--------Cancelar/Voltar----------+
```
`Screen` ganha `ReceiveImage` e `ConfirmImage`. A sessão do servidor pertence ao fluxo das duas telas e só termina ao
Aplicar, ao sair pela tela Receber imagem, no Home, no segundo plano ou no prazo esgotado de vez.

### 11. Edição distinta de inatividade
Enquanto a página está aberta e `document.visibilityState === "visible"`, ela manda `POST /ping` (rota da sessão, comum a
todas as funções) a cada 30 s com
`idle`, os segundos desde o último `pointerdown`, `wheel`, `keydown` ou troca de arquivo. A TV conta como edição: o
carregamento da página, um envio, ou um aviso com `idle` < 120. O prazo de 5 minutos conta da última edição; a tela
mostra "Celular conectado · editando…" até 75 s depois dela (dois avisos perdidos). Ao esgotar o prazo, a sessão gera
um código novo e reabre o servidor, no máximo 3 vezes seguidas sem edição; na quarta, desliga e mostra "Tempo esgotado".
O relógio aparece só no último minuto. Os prazos são absolutos (contados do último quadro), para o laço não os empurrar
para a frente a cada volta.

### 12. Validação em duas pontas
- **Celular:** formato decodificável, lado menor ≥ 64 e o aviso de tamanho. Isso dá resposta imediata.
- **TV:** `Content-Length` ≤ 9 MB, cabeçalho QOI (`qoif`, canais 3/4), exatamente 1920x1080, decodificação completa e
  consistente com o tamanho declarado. O alfa é descartado, compondo sobre preto.

### 13. HEIC por WebAssembly, carregado só quando preciso
Os aparelhos da casa gravam fotos em HEIC, e só o Safari as decodifica. A página tenta primeiro o decodificador do
navegador (`createImageBitmap`, depois `<img>`); se falhar e o arquivo for HEIC/HEIF (tipo ou extensão, ou a marca
`ftypheic`/`ftypmif1` no começo do arquivo), ela carrega `/lib/heic.js`, que busca `/lib/heic.wasm`, e decodifica com a libheif
(com a libde265 para o HEVC), que aplica a rotação gravada no arquivo (`irot`/`imir`). O RGBA vira um `ImageData` e
segue o caminho de sempre (cópia de trabalho de 4 MP, edição, envio). Uma foto de 12 MP ocupa ~50 a 100 MB do navegador
por um instante e leva ~1 a 3 s; a página mostra "Preparando a imagem…".

Os dois arquivos (~1,5 MB) são gerados por `scripts/build-heic-wasm.sh`: baixa os fontes da libheif e da libde265 em
versões fixas, confere o SHA-256, compila com o Emscripten de uma imagem `emscripten/emsdk` fixada, no mesmo runtime de
contêiner do `container-build.sh`, e grava `crates/lume-core/assets/companion/heic/heic.js` e `heic.wasm`, com um
`SOURCES.txt` (versões, URLs, hashes e as opções de build). Os arquivos são commitados, como os subconjuntos de fontes, e o
build do APK só os copia para `assets/companion/` (uma cópia no APK, não dentro de cada `liblume.so`), de onde o servidor
os lê pelo leitor de assets que o papel de parede já usa. No `lume-desktop`, são lidos do repositório.

**Licença:** libheif e libde265 são LGPL-3.0, compatível com a GPL-3.0-or-later do Lume. Entram na tela Sobre (à mão, porque
a lista gerada só tem crates) e em `docs/`, com o `SOURCES.txt` e o script, o que entrega o fonte correspondente.
**Patentes:** o HEVC é coberto por patentes (pools Access Advance e Via LA); por isso navegadores não o trazem por
software. Incluir o decodificador é decisão do autor, registrada nesta change. **Alternativas:** binário pronto do npm
(`libheif-js`), descartado por não ser compilado aqui; compilar a cada build do APK, descartado pelo peso do Emscripten.

### 14. Android TV 9
Muitas TVs no Brasil ainda estão no Android 9 (o firmware original da TCL de referência é o 9), então o fluxo inteiro
tem de funcionar nele. O `minSdk` já é 28, e o fluxo só usa o que o NDK tem desde a API 28: `std::net` (sockets), `SIOCGIFCONF` para o IPv4
(no Android 9 `getifaddrs` também funcionaria, mas o mesmo código serve ao 11), `AAssetManager` para o `heic.js`/`heic.wasm`
e `/dev/urandom`. Escutar numa porta acima de 1024 não pede permissão além de `INTERNET`, e o tráfego em texto claro só é
restrito para conexões de saída, que o Lume não abre. As duas TVs de teste estão no Android 11 (firmware global), então o
Android 9 é verificado no emulador `tv_api28`: o IP dele (10.0.2.15) não é alcançável do computador, e o teste usa
`adb forward tcp:<porta> tcp:<porta>` e a página em `localhost`. A memória (a imagem recebida, 6,2 MB) e a textura seguem
o caminho dos papéis do catálogo, que já roda no Android 9.

## Risks / Trade-offs

| Risco | Mitigação |
|---|---|
| Rede de convidados ou isolamento de clientes (o celular não alcança a TV) | O endereço e o código aparecem para digitar, e a página não abre. A tela dá a dica "o celular precisa estar na mesma rede". Não há como contornar sem um servidor externo, que está fora do escopo. |
| HTTP sem TLS (outra pessoa na rede poderia ver a imagem passar) | Fica só na rede local, enquanto as telas estão abertas, e só a imagem trafega. Documentar em `docs/wallpaper.md`. |
| O que se vê no celular não é a tela da TV (brilho, cores, a barra superior por cima) | A tela Confirmar mostra o resultado na própria TV antes de aplicar, e reenviar é imediato. |
| Memória do celular com fotos grandes | A cópia de trabalho é reduzida a ≤ 4 MP logo ao abrir; o canvas de 1920x1080 só existe no envio. |
| Navegador sem `createImageBitmap` com `imageOrientation` ou sem `ctx.filter` (Safari antigo) | Fallback por `<img>` e `canvas`, que respeitam o EXIF nos navegadores atuais; desfoque em JS sobre a imagem a 1/8. |
| HEIC no Chrome do Android e do computador | Decodificador libheif em WebAssembly, carregado só quando o navegador não abre a foto. Se ele falhar (arquivo estranho, pouca memória), a página explica como salvar em JPEG. |
| Patentes do HEVC no decodificador da página | Decisão do autor registrada; o decodificador é baixado só por quem escolhe um HEIC num navegador sem suporte. |
| Memória do celular ao decodificar HEIC de 12 MP ou mais | A decodificação acontece uma vez, e o RGBA é reduzido logo à cópia de trabalho de 4 MP e liberado. |
| Página esquecida aberta mantendo o servidor | Sem interação há 2 minutos, os avisos deixam de contar como edição, e o prazo corre. |
| Firewall ou política do fabricante bloqueando listen | Testado na TCL e na AIWA; se falhar, a tela mostra o erro de "não foi possível abrir a porta". |
| Android TV 9 sem TV física de teste (as duas estão no 11) | Fluxo completo no emulador `tv_api28` com `adb forward` (decisão 14). |
| Porta 8642 em uso por outro app | Fallback para a porta 0, que fica no QR. |
| Imagem do usuário clara demais sob a barra superior | É decisão do autor não ter sombra. A tela Confirmar mostra o resultado antes de aplicar. |

## Migration Plan

Não há migração de dados para quem não usou a versão de desenvolvimento. `config.v1` ganha chaves novas e opcionais.
Uma versão anterior que leia `wallpaper=custom` cai na Aurora (comportamento atual de `from_str`). Chaves `custom_fit`,
`custom_zoom`, `custom_x`, `custom_y`, `custom_tile` e `custom_scale` gravadas pela primeira versão desta change são
ignoradas, e o `custom.qoi` dela (a fonte, não o resultado) não tem 1920x1080: a TV o trata como imagem ausente e volta à
Aurora. O rollback é reinstalar a versão anterior; `files/wallpaper/custom.qoi` fica órfão e inofensivo.

## Open Questions

- Porta preferida definitiva (8642 é provisória). Não muda o design.
- Versões exatas da libheif e da libde265 (as últimas estáveis na hora do script).

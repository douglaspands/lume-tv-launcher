# platform-compatibility Specification

## Purpose
Define em quais dispositivos o lume roda, quais permissões ele pede e quais requisitos de distribuição atende para o GitHub e a Play Store.

## Requirements

### Requirement: Versões do Android suportadas
O Lume SHALL instalar e funcionar em Android TV 9 (API 28) a Android TV 11 (API 30), e SHALL ser compilado contra um nível de API alvo aceito pela Play Store no momento do release. O botão de entradas e a engrenagem da barra superior SHALL funcionar nas duas pontas dessa faixa. As telas Receber imagem e Confirmar e o servidor do celular (capacidade `phone-companion`) SHALL funcionar nas duas pontas também, usando só o que o NDK oferece desde a API 28 (sockets, `SIOCGIFCONF`, `AAssetManager`, `/dev/urandom`), sem Java novo.

#### Scenario: Android TV 9
- **WHEN** o APK é instalado num emulador Android TV 9 (API 28)
- **THEN** a instalação é bem-sucedida, todas as telas funcionam e a engrenagem abre as configurações do Android

#### Scenario: Android TV 11
- **WHEN** o APK é instalado na TCL 55C715 ou na AIWA SMART TV (Android TV 11), ou em emulador Android TV 11
- **THEN** a instalação é bem-sucedida, o catálogo lista os apps instalados e o botão de entradas abre o menu de entradas da TV

#### Scenario: Visibilidade da ação de entradas no Android 11
- **WHEN** o Lume consulta o sistema se existe quem atenda a `com.android.tv.action.VIEW_INPUTS` no Android 11
- **THEN** a consulta enxerga a activity de entradas do fabricante, porque o manifesto declara a ação em `<queries>`

#### Scenario: Versão anterior
- **WHEN** alguém tenta instalar o APK em Android 8.1 ou anterior
- **THEN** o sistema recusa a instalação

#### Scenario: Imagem do celular no Android TV 9
- **WHEN** a tela Receber imagem é aberta no emulador Android TV 9 (API 28) e o navegador do computador abre a página pela porta encaminhada (`adb forward`)
- **THEN** a tela mostra o QR com o IPv4 da interface do emulador, a página abre com o código, o envio chega à tela Confirmar e Aplicar grava "Minha imagem", como no Android 11

### Requirement: Arquiteturas de CPU
O APK de release SHALL conter código nativo para `armeabi-v7a` e `arm64-v8a`. O APK de debug SHALL conter também `x86_64`, para rodar em emuladores Android TV. Todas as bibliotecas nativas SHALL ter alinhamento de página de 16 KB.

#### Scenario: Emulador x86_64
- **WHEN** o APK de debug é instalado num emulador Android TV 11 x86_64
- **THEN** o lume é executado com a biblioteca `x86_64`

#### Scenario: Release sem x86_64
- **WHEN** o APK de release é inspecionado
- **THEN** ele contém apenas as bibliotecas `armeabi-v7a` e `arm64-v8a`

#### Scenario: Userspace 32 bits
- **WHEN** o lume é instalado em uma TV com CPU de 64 bits e userspace de 32 bits
- **THEN** o lume é executado com a biblioteca `armeabi-v7a`

### Requirement: Gráficos mínimos
O lume SHALL funcionar em GPUs que suportam apenas OpenGL ES 2.0.

#### Scenario: GPU só com GLES 2.0
- **WHEN** o dispositivo reporta OpenGL ES 2.0 como versão máxima
- **THEN** todas as telas são renderizadas corretamente

### Requirement: Permissões mínimas
O lume SHALL NOT solicitar nenhuma permissão perigosa (runtime), permissão de localização nem `QUERY_ALL_PACKAGES`. As permissões declaradas SHALL ser apenas permissões normais, concedidas na instalação. A primeira é `REQUEST_DELETE_PACKAGES`, usada só para abrir o desinstalador do sistema a pedido do usuário. A segunda é `INTERNET`, exigida pelo Android para abrir o socket que lista as interfaces de rede (a conexão em Informações da TV) e para escutar na rede local enquanto as telas Receber imagem e Confirmar estão abertas e por até 10 s depois de Aplicar, só para avisar a página (capacidade `phone-companion`); ela SHALL NOT ser usada para abrir conexões de saída nem para escutar fora disso. A visibilidade dos apps SHALL ser declarada por consultas de intent (LEANBACK_LAUNCHER e LAUNCHER).

#### Scenario: Inspeção do manifesto
- **WHEN** o manifesto final do APK é inspecionado (`aapt dump permissions`)
- **THEN** aparecem só `android.permission.REQUEST_DELETE_PACKAGES` e `android.permission.INTERNET`, sem `QUERY_ALL_PACKAGES` nem permissão de localização

#### Scenario: Nenhum tráfego de rede
- **WHEN** o lume é usado em todas as telas, inclusive as Ferramentas, exceto a tela Receber imagem
- **THEN** o processo não abre nenhuma conexão de rede nem escuta em nenhuma porta

#### Scenario: Só escuta, nunca conecta
- **WHEN** a tela Receber imagem está aberta
- **THEN** o processo escuta numa única porta TCP e não abre nenhuma conexão de saída

#### Scenario: Desinstalar no Android 9 e 11
- **WHEN** o usuário escolhe "Desinstalar" no menu de contexto no Android TV 9 ou 11
- **THEN** o diálogo de desinstalação do sistema aparece, sem nenhum pedido de permissão

### Requirement: Requisitos de app de TV
O lume SHALL declarar o recurso leanback como suportado, declarar o toque como não obrigatório e fornecer um banner de TV de 320x180 com a marca lume.

#### Scenario: Aparece no Android TV
- **WHEN** o lume é instalado e outro launcher está ativo
- **THEN** o lume aparece na lista de apps do sistema com o seu banner

### Requirement: Artefatos de distribuição
Cada release SHALL produzir um APK assinado (para o GitHub) e um AAB (para a Play Store), assinados com a mesma chave de app e com o mesmo identificador de pacote, de modo que um APK do GitHub possa ser atualizado por uma versão mais nova de qualquer um dos canais.

#### Scenario: Build local em container
- **WHEN** alguém com apenas um runtime de containers (wslc ou Docker) roda `scripts/container-build.sh release` num clone do repositório
- **THEN** o APK e o AAB são gerados em `dist/`, com as mesmas versões de ferramentas usadas pelo CI

#### Scenario: Release por tag
- **WHEN** uma tag `vX.Y.Z` é publicada no repositório
- **THEN** o CI gera o APK e o AAB assinados, com `versionName` X.Y.Z, anexa ao release do GitHub o APK chamado `lume-X.Y.Z.apk` e dá ao release o título "Lume vX.Y.Z"

#### Scenario: Atualização entre canais
- **WHEN** o lume 0.1.0 do GitHub está instalado e o usuário instala a 0.2.0 vinda do mesmo processo de assinatura
- **THEN** a atualização é aceita, e a configuração do usuário é mantida

### Requirement: Versão do build
A versão de cada build SHALL vir do próprio build, sem editar arquivos do repositório, e SHALL ser a mesma no Android (`versionName`), na interface e no log do app.

- Num build de release, a versão SHALL ser a tag sem os caracteres "v" do início (`v0.3.0`, `vv0.3.0` e `0.3.0` dão `0.3.0`), e o `versionCode` SHALL ser calculado a partir dos números X.Y.Z (X·10000 + Y·100 + Z), ignorando um sufixo de pré-release.
- Num build local, a versão SHALL ser o hash curto do commit atual. Se algum arquivo rastreado pelo git tiver mudança não commitada, a versão SHALL terminar em `-dirty`. O `versionCode` SHALL ser o número de commits do histórico do commit atual.
- Sem informação de git disponível, a versão SHALL ser `dev` e o `versionCode` SHALL ser 1.
- A versão do pacote Rust (`Cargo.toml`) SHALL NOT influenciar a versão do build.

#### Scenario: Tag com "v"
- **WHEN** o CI gera o release a partir da tag `v0.3.0`
- **THEN** o `versionName` é `0.3.0` e o `versionCode` é 300

#### Scenario: Tag com "v" repetido
- **WHEN** a versão informada ao build é `vv0.3.1`
- **THEN** o `versionName` é `0.3.1`

#### Scenario: Build local limpo
- **WHEN** alguém gera o APK localmente no commit `4d8438b`, sem mudanças não commitadas, num histórico de 6 commits
- **THEN** o `versionName` é `4d8438b`, o `versionCode` é 6 e o arquivo em `dist/` leva `4d8438b` no nome

#### Scenario: Build local com mudanças
- **WHEN** alguém gera o APK localmente no commit `4d8438b` com um arquivo rastreado modificado e não commitado
- **THEN** o `versionName` é `4d8438b-dirty`

#### Scenario: Sem git
- **WHEN** o código é compilado fora de um repositório git e sem versão informada
- **THEN** o `versionName` é `dev` e o `versionCode` é 1

# Lume

**Uma tela inicial rápida e leve para Android TV.** O Lume substitui o launcher que veio na sua TV por um launcher simples, feito para o controle remoto, que usa pouquíssima memória, deixando mais memória para os seus apps de streaming.

[English](README.md) | Português

> **Situação:** o Lume está em desenvolvimento. A versão atual é a **0.3.0**, já publicada na [página de Releases](https://github.com/douglaspands/lume-tv-launcher/releases). Os passos abaixo mostram como instalá-la.

<p align="center">
  <img src="docs/screenshots/home-pt-BR.png" alt="Tela Início com o dock" width="49%">
  <img src="docs/screenshots/apps-pt-BR.png" alt="Tela Apps" width="49%">
</p>
<p align="center">
  <img src="docs/screenshots/settings-pt-BR.png" alt="Configurações" width="49%">
  <img src="docs/screenshots/editor-pt-BR.png" alt="Edição dos favoritos" width="49%">
</p>

**O que você ganha**

- Uma tela inicial com um dock flutuante para os seus apps favoritos: o ícone em foco cresce e os vizinhos abrem espaço.
- Uma tela "Apps" com tudo o que está instalado na TV.
- Tudo funciona com o controle remoto comum (setas, OK, Voltar, Home).
- Português (Brasil) e inglês, seguindo o idioma da TV.
- Nenhum acesso à internet: o Lume nunca se conecta à rede e não pede permissões perigosas.

**O que você precisa**

- Uma TV com **Android TV 9, 10 ou 11**.
- Um computador (Windows, macOS ou Linux) na **mesma rede Wi-Fi** da TV.
- Uns 10 minutos.

---

## Instalar o Lume na TV

Você vai enviar o app do computador para a TV com uma ferramenta pequena chamada **adb**. Só precisa fazer isso uma vez.

### 1. Baixe o Lume

1. Abra a [página **Releases**](https://github.com/douglaspands/lume-tv-launcher/releases) deste projeto no GitHub.
2. Na versão mais nova, baixe o arquivo com nome parecido com **`lume-0.3.0.apk`**.
3. Anote onde ele foi salvo (por exemplo, a pasta Downloads).

### 2. Instale o adb no computador

1. Baixe o **SDK Platform-Tools** do Google para o seu sistema em <https://developer.android.com/tools/releases/platform-tools>.
2. Descompacte numa pasta fácil, por exemplo `C:\platform-tools` no Windows ou `~/platform-tools` no macOS/Linux.
3. Abra um terminal nessa pasta:
   - **Windows:** abra a pasta, clique na barra de endereço, digite `cmd` e tecle Enter.
   - **macOS:** abra o *Terminal*, digite `cd ~/platform-tools` e tecle Enter.
   - **Linux:** abra um terminal e digite `cd ~/platform-tools`.

Para conferir, digite `adb version` (no Windows) ou `./adb version` (no macOS/Linux). Deve aparecer um número de versão.

> No macOS e no Linux, coloque `./` antes de cada comando `adb` abaixo (por exemplo, `./adb devices`).

### 3. Ative a depuração na TV

1. Na TV, abra **Configurações → Preferências do dispositivo → Sobre**.
2. Selecione **Compilação** (ou *Build do Android TV OS*) e pressione OK **7 vezes**, até aparecer que você é um desenvolvedor.
3. Volte para **Preferências do dispositivo → Opções do desenvolvedor**.
4. Ative a **Depuração USB** e também a **Depuração de rede** / **Depuração ADB por rede**, se aparecer.

Os nomes dos menus mudam um pouco entre as marcas de TV.

### 4. Descubra o endereço da TV

Na TV, abra **Configurações → Rede e Internet**, selecione a sua conexão e anote o **endereço IP**, algo como `192.168.1.50`.
(Algumas TVs também mostram o endereço nas *Opções do desenvolvedor*, perto da chave de depuração de rede.)

### 5. Conecte e instale

No terminal aberto no passo 2, digite (troque o endereço pelo seu):

```
adb connect 192.168.1.50:5555
```

Vai aparecer uma mensagem na TV perguntando *"Permitir a depuração USB?"*. Escolha **Sempre permitir** e **OK**. Depois rode o mesmo comando de novo e confira se a TV aparece na lista:

```
adb devices
```

Você deve ver a sua TV com a palavra `device` ao lado. Agora instale o Lume (use o caminho real do arquivo que você baixou):

```
adb install "C:\Users\Voce\Downloads\lume-0.3.0.apk"
```

Quando aparecer `Success`, o Lume está instalado.

### 6. Faça do Lume a sua tela inicial

1. Aperte o botão **Home** do controle. O Android pode perguntar qual app usar como tela inicial: escolha **Lume** e **Sempre**.
2. Se nada foi perguntado, ou se o Home ainda abre o launcher antigo, rode:

   ```
   adb shell cmd package set-home-activity app.lume.tv/.LumeActivity
   ```

3. Algumas TVs ignoram isso e continuam abrindo o launcher delas. Como último recurso, dá para desligar o launcher original (ele não é apagado e dá para ligar de novo):

   ```
   adb shell pm disable-user --user 0 com.google.android.tvlauncher
   ```

### Voltar ao launcher original

Remova o Lume e, se você desligou o launcher original, ligue-o de novo:

```
adb shell pm enable com.google.android.tvlauncher
adb uninstall app.lume.tv
```

### Algo deu errado?

| Problema | O que tentar |
|---|---|
| `adb: command not found` / `'adb' não é reconhecido` | Você não está na pasta platform-tools. Abra o terminal dentro dela (passo 2). |
| `failed to connect` / `unable to connect` | Confira o endereço da TV, se TV e computador estão na mesma rede e se a depuração está ligada. Tente desligar e ligar a TV. |
| `device unauthorized` | Olhe a tela da TV e aceite o aviso, depois rode `adb devices` de novo. |
| `INSTALL_FAILED_OLDER_SDK` | A sua TV tem Android anterior ao 9. O Lume não roda nela. |
| `INSTALL_FAILED_UPDATE_INCOMPATIBLE` | Há outra versão do Lume instalada. Rode `adb uninstall app.lume.tv` e instale de novo. |
| O Home abre o launcher antigo | Repita o passo 6, opções 2 e 3. |

---

## Para desenvolvedores

O Lume é escrito em Rust. Ele desenha toda a interface com OpenGL ES 2.0 (sem Views, WebView, AndroidX ou Kotlin) e libera toda a memória de GPU enquanto outro app está em primeiro plano.

### Estrutura do repositório

| Caminho | Conteúdo |
|---|---|
| `crates/lume-css` | Parser do subconjunto de CSS dos temas (veja [`docs/theme-format.md`](docs/theme-format.md)) |
| `crates/lume-strings` | Transforma as traduções `strings.xml` do Android em tabelas Rust no build |
| `crates/lume-core` | Lógica independente de plataforma: telas, foco, tema, texto e renderer de GPU, i18n, catálogo, persistência |
| `crates/lume-app` | A biblioteca Android (`liblume.so`): NativeActivity, EGL, JNI, laço de eventos |
| `tools/lume-desktop` | Visualizador e gerador de screenshots: roda o launcher sem TV |
| `android/` | Projeto Gradle mínimo: manifesto, `LumeActivity`, `LumeBridge`, recursos, empacotamento |
| `themes/lume/` | O tema padrão (MIT) |
| `docker/`, `scripts/` | Build reproduzível em container, verificação do APK, geradores de fontes |
| `docs/` | Formato do tema, guia de tradução, notas de dispositivo |
| `openspec/` | Specs, design e lista de tarefas da versão atual |

### Rodar no computador

Você não precisa de uma TV para ver a interface. Este comando abre uma janela em que o teclado é o controle remoto (setas, **Enter** = OK, **Esc** = Voltar, **H** = Home):

```
cargo run -p lume-desktop
cargo run -p lume-desktop -- --locale pt-BR --size 1920x1080
```

Ele usa o mesmo código OpenGL ES 2.0 da TV (o renderer por software do Mesa basta). Para tirar fotos de uma tela (`home`, `apps`, `settings`, `about`, `editor`):

```
cargo run -p lume-desktop -- --screenshot home home.png
cargo run -p lume-desktop -- --size 1920x1080 --keys down,right,right --screenshot home foco.png
```

`--keys` aperta teclas antes (`up down left right ok back home`), `--locale` escolhe o idioma e `--pseudo-locale` alonga todos os textos em 40% para achar layouts que não cabem.

### Build com container (recomendado)

Você só precisa de um runtime de containers. Não precisa de Rust, NDK nem Android SDK na sua máquina.

A ordem também é a prioridade: o script usa o primeiro runtime que encontrar (a não ser que você defina `LUME_CONTAINER`):

1. **wslc:** a ferramenta que vem com o WSL no Windows.
2. **Docker:** com BuildKit (`docker buildx`).
3. **Podman:** versão 5 ou mais nova.

Se nenhum dos três estiver instalado, o script avisa e sugere alternativas: atualizar o WSL para ter o `wslc`, instalar o Docker ou o Podman, apontar `LUME_CONTAINER` para outro runtime compatível com o Docker, ou compilar sem containers instalando Rust, Android NDK/SDK e `cargo-ndk` e seguindo o `docker/Dockerfile` à mão.

```
scripts/container-build.sh debug      # APK com arm64, armv7 e x86_64
scripts/container-build.sh release    # APK e AAB com arm64 e armv7
```

O resultado vai para `dist/`. O primeiro build baixa o Android SDK/NDK e o Rust e demora; os seguintes reaproveitam o cache. Use `LUME_CONTAINER` para forçar um runtime.

Instale na TV ou no emulador com `adb install -r dist/lume-<version>-debug.apk`.

**Assinar um release.** Sem chave, o release é assinado com a chave de debug e se chama `...-release-debugsigned.apk`: serve para você instalar, não para publicar. Para assinar com a sua chave, defina isto antes de rodar o script (os valores vão para o build como segredos e nunca ficam na imagem):

```
export LUME_KEYSTORE=/caminho/para/lume.jks
export LUME_KEYSTORE_PASSWORD=...
export LUME_KEY_ALIAS=...
export LUME_KEY_PASSWORD=...
scripts/container-build.sh release    # dist/lume-<version>-release-debugsigned.apk
```

Guarde o keystore fora do repositório e faça backup: a mesma chave precisa assinar todas as atualizações. A versão vem do build, não de um arquivo: a tag `v0.3.0` vira `0.3.0` (`LUME_VERSION_NAME=v0.3.0` faz o mesmo) e um build local usa o hash curto do commit (`4d8438b`, com `-dirty` se houver mudança não commitada em arquivo rastreado; `dev` sem git), então o arquivo se chama `dist/lume-4d8438b-release-debugsigned.apk`.

**Conferir um APK.** `scripts/verify-apk.py dist/lume-<version>-release-debugsigned.apk` confere o tamanho (6 MB), as ABIs, o alinhamento de 16 KB das bibliotecas nativas e que o app não pede `INTERNET` nem `QUERY_ALL_PACKAGES`.

### Build sem container

Você precisa de:

- Rust (a versão e os alvos Android vêm do `rust-toolchain.toml`; instale o [rustup](https://rustup.rs) e ele os usa).
- JDK 17, Android SDK platform 36 e build-tools 36.0.0.
- Android NDK `28.2.13676358` (defina `ANDROID_NDK_HOME`).
- `cargo install cargo-ndk --version 4.1.2 --locked`

Depois:

```
cd android
./gradlew assembleDebug      # ou assembleRelease
```

O Gradle chama o `cargo ndk` por você e alinha as bibliotecas nativas a páginas de 16 KB.

### Rodar os testes

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Os testes de gráficos precisam de um EGL funcionando (Mesa no Linux); eles são pulados quando não há.

### Releases

Enviar uma tag como `v0.1.0` (ou `v0.1.0-rc1` para uma pré-versão) roda o `.github/workflows/release.yml`: ele gera o APK e o AAB assinados com o mesmo Dockerfile, roda as verificações acima e anexa o APK a um release do GitHub. Ele precisa destes segredos no repositório: `LUME_KEYSTORE_BASE64` (o keystore em base64), `LUME_KEYSTORE_PASSWORD`, `LUME_KEY_ALIAS` e `LUME_KEY_PASSWORD`.

### Traduções

As traduções são arquivos `strings.xml` padrão do Android em `android/app/src/main/res/values-<idioma>/`. Veja [`docs/translations.md`](docs/translations.md).

### Papel de parede

O Lume desenha uma fotografia de aurora boreal atrás de todas as telas. Como ela é processada no build, como trocá-la e onde ficam a origem e a licença: [`docs/wallpaper.md`](docs/wallpaper.md).

### Contribuição, licença e marca

- O código é licenciado sob **GPL-3.0-or-later** ([`LICENSE`](LICENSE)); os temas sob **MIT** ([`themes/LICENSE`](themes/LICENSE)).
- O nome e o logo "lume" não são licenciados: veja [`TRADEMARKS.md`](TRADEMARKS.md).
- Contribuições exigem um CLA assinado: veja [`CONTRIBUTING.md`](CONTRIBUTING.md) e [`CLA.md`](CLA.md).

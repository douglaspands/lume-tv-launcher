# installation-docs Specification

## Purpose
Define o que os READMEs (em 5 idiomas) trazem para instalar o Lume na TV: requisitos, contexto sobre o adb, as duas opções de instalação (assistente de IA e manual) e a paridade entre idiomas.

## Requirements

### Requirement: Requisitos de instalação nos READMEs
Cada README (`README.md`, `README.pt-BR.md`, `README.es.md`, `README.id.md`, `README.ru.md`) SHALL abrir a seção de instalação com a lista de requisitos: uma TV com Android TV 9 ou superior na mesma rede do computador, o adb instalado no computador, a depuração USB ou de rede ativada na TV e o APK do Lume.

#### Scenario: Requisitos visíveis
- **WHEN** alguém abre a seção de instalação de qualquer README
- **THEN** vê a lista de requisitos antes das duas opções de instalação

### Requirement: Contexto sobre o adb
Cada README SHALL explicar em poucas linhas o que é o adb (Android Debug Bridge, a ferramenta de linha de comando do Google que conversa com a TV pela rede e instala apps nela) e por que é necessário (a TV não instala o Lume pela loja).

#### Scenario: Leitor sem experiência com adb
- **WHEN** alguém abre a seção de instalação de qualquer README
- **THEN** encontra a explicação do que é o adb antes dos passos

### Requirement: APK indicado pelo repositório
Cada README SHALL indicar o APK apenas pelo endereço do repositório, `https://github.com/douglaspands/lume-tv-launcher`, sem número de versão.

#### Scenario: Link do APK
- **WHEN** o README manda baixar o APK
- **THEN** aponta para `https://github.com/douglaspands/lume-tv-launcher` e não cita versão nem nome de arquivo com versão

### Requirement: Duas opções de instalação
Cada README SHALL oferecer duas opções, nesta ordem: a Opção 1, por um assistente de IA (um agente de linha de comando capaz de rodar comandos no computador), com um prompt pronto para colar; e a Opção 2, manual, com os 6 passos atuais, sem citar versão do APK. O último recurso do passo 6 SHALL desativar o launcher original junto com o serviço de recomendações (`com.google.android.tvrecommendations`), e "Voltar ao launcher original" SHALL reativar os dois.

#### Scenario: Escolha da opção
- **WHEN** o leitor chega à seção de instalação
- **THEN** encontra a Opção 1 (assistente de IA) antes da Opção 2 (manual), e as duas levam ao mesmo resultado

#### Scenario: Desativar o launcher original
- **WHEN** o leitor chega ao último recurso do passo 6 da Opção 2
- **THEN** o README manda rodar `adb shell pm disable-user --user 0 com.google.android.tvlauncher` e `adb shell pm disable-user --user 0 com.google.android.tvrecommendations`, e explica que o serviço de recomendações reativa o launcher do Android e congela o Lume

#### Scenario: Reverter
- **WHEN** o leitor segue "Voltar ao launcher original"
- **THEN** o README manda rodar `pm enable` para `com.google.android.tvlauncher` e para `com.google.android.tvrecommendations` antes de `adb uninstall app.lume.tv`

### Requirement: Prompt para assistente de IA
O prompt da Opção 1 SHALL estar no idioma de cada README e SHALL instruir o assistente a verificar o adb, baixar o APK do repositório, pedir o IP da TV, conectar, instalar com `adb install`, confirmar `Success` e definir o Lume como Home com `set-home-activity`. Se o Home continuar abrindo o launcher original, SHALL pedir a confirmação do usuário e desativar o launcher e o serviço de recomendações juntos, nunca só o launcher.

#### Scenario: Passos do prompt
- **WHEN** o assistente segue o prompt
- **THEN** verifica o adb (e explica como instalá-lo, se faltar), baixa o APK, pede o IP e a confirmação da depuração, roda `adb connect` e `adb devices` lembrando o usuário de aceitar o aviso na TV, instala e confirma `Success`

#### Scenario: Endereço da TV
- **WHEN** o assistente precisa conectar
- **THEN** pede o IP ao usuário em vez de adivinhar

#### Scenario: Launcher original insiste
- **WHEN** depois do `set-home-activity` o Home ainda abre o launcher original e o usuário confirma
- **THEN** o assistente desativa `com.google.android.tvlauncher` e `com.google.android.tvrecommendations` com `pm disable-user --user 0`, e explica como reverter com `pm enable`

### Requirement: Paridade entre idiomas
Os 5 READMEs SHALL ter as mesmas seções de instalação, na mesma ordem e com os mesmos comandos; só o texto muda de idioma.

#### Scenario: Conferência
- **WHEN** a seção de instalação de um README muda
- **THEN** os outros 4 recebem a mesma mudança na mesma change

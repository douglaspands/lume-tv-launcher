# phone-companion Specification

## Purpose
Liga o celular do usuário ao Lume pela rede local, sem app instalado: a TV abre por pouco tempo um servidor HTTP mínimo,
mostra um código QR e serve uma página do Lume em que o celular edita e envia conteúdo (hoje, o papel de parede).

## Requirements

### Requirement: Servidor local só com as telas abertas
O Lume SHALL escutar na rede local só com as telas Receber imagem ou Confirmar abertas, também durante a confirmação (para reenviar). O servidor SHALL ser desligado 10 s depois de Aplicar, ao sair do fluxo (Voltar ou Cancelar na Receber imagem, Home, segundo plano) e no prazo esgotado de vez; desligado, nenhuma porta do Lume fica aberta.

#### Scenario: Abrir e sair da tela
- **WHEN** o usuário abre a tela Receber imagem e depois pressiona Voltar
- **THEN** o endereço mostrado responde enquanto a tela está aberta e recusa conexão depois de Voltar

#### Scenario: Lume vai para segundo plano
- **WHEN** a tela Receber imagem ou a tela Confirmar está aberta e outro app passa ao primeiro plano
- **THEN** o servidor é desligado antes de o Lume entrar em segundo plano

#### Scenario: Imagem recebida
- **WHEN** a TV aceita uma imagem válida
- **THEN** responde ao celular com sucesso, abre a tela Confirmar e mantém o servidor ligado

#### Scenario: Aplicar
- **WHEN** o usuário pressiona Aplicar na tela Confirmar
- **THEN** o próximo aviso da página recebe "aplicada", um envio novo é recusado, e 10 s depois o servidor é desligado e nenhuma porta do Lume fica aberta

### Requirement: Endereço e código de acesso
Ao abrir a tela, o Lume SHALL escolher o IPv4 privado da interface ativa (cabo antes de Wi-Fi), uma porta (a preferida ou, ocupada, uma livre) e um código novo de 4 caracteres sem ambiguidade (sem 0/O, 1/I/L). O QR SHALL levar o endereço já com o código. Todo envio sem o código certo SHALL ser recusado, e 5 códigos errados SHALL desligar o servidor.

#### Scenario: QR com código
- **WHEN** a tela Receber imagem é aberta numa TV com IP privado
- **THEN** o QR codifica `http://<ip>:<porta>/wallpaper/?c=<código>` e a tela mostra o endereço sem o código (`<ip>:<porta>/wallpaper`) e, à parte, o código

#### Scenario: Digitado à mão
- **WHEN** o usuário digita o endereço sem o código no navegador
- **THEN** a página pede o código de 4 caracteres e só libera o envio com o código certo

#### Scenario: Código errado repetido
- **WHEN** o servidor recebe 5 envios com código errado
- **THEN** o servidor é desligado e a tela mostra "Muitas tentativas" com o botão "Gerar novo código"

#### Scenario: Código renovado
- **WHEN** o usuário sai e volta à tela Receber imagem
- **THEN** o código e o QR são diferentes dos anteriores

### Requirement: Edição distinta de inatividade
Enquanto a página estiver aberta e visível, ela SHALL avisar a TV a cada 30 s, dizendo há quantos segundos foi a última interação nela. A TV SHALL contar como edição o carregamento da página, um envio e um aviso com interação nos últimos 2 minutos; a página escondida não avisa. A tela Receber imagem SHALL mostrar "Celular conectado · editando…" até 75 s depois da última edição.

#### Scenario: Escolhendo e ajustando a foto
- **WHEN** o usuário passa 10 minutos escolhendo e ajustando a foto no celular, sem enviar
- **THEN** o código não expira, e a TV mostra "Celular conectado · editando…"

#### Scenario: Celular esquecido com a página aberta
- **WHEN** a página fica aberta no celular sem nenhum toque
- **THEN** depois de 2 minutos os avisos deixam de contar como edição, e o prazo de inatividade começa a correr

#### Scenario: Tela do celular apagada
- **WHEN** o celular apaga a tela com a página aberta
- **THEN** a página para de avisar, e a TV deixa de mostrar "Celular conectado" depois de 75 s

### Requirement: Prazo de inatividade
O servidor SHALL esperar 5 minutos depois da última edição (ou da abertura da tela). A tela Receber imagem SHALL mostrar o tempo restante só no último minuto ("O código expira em 0:45"); antes disso, mostra só o estado. Ao esgotar, a TV SHALL gerar um código e um QR novos sozinha, no máximo 3 vezes seguidas sem edição; na quarta, SHALL desligar o servidor e mostrar "Tempo esgotado" com o botão "Gerar novo código".

#### Scenario: Último minuto
- **WHEN** faltam 45 s para o prazo sem nenhum celular conectado
- **THEN** a tela mostra "O código expira em 0:45", atualizado a cada segundo

#### Scenario: Código novo automático
- **WHEN** o prazo se esgota pela primeira vez sem edição
- **THEN** a tela mostra um código e um QR novos, sem o usuário apertar nada, e o prazo recomeça

#### Scenario: Tempo esgotado de vez
- **WHEN** o prazo se esgota pela quarta vez seguida sem edição
- **THEN** o servidor é desligado e a tela mostra "Tempo esgotado" com o botão "Gerar novo código"

### Requirement: A página sabe da decisão da TV
Depois de um envio aceito, a página SHALL perguntar à TV a cada 2 s o que ela fez com a foto, na resposta ao aviso (`/ping`): aguardando, aplicada ou não usada. Aplicada, a página SHALL encerrar: some a edição, aparece "Pronto! O papel de parede está na TV." e "Você já pode fechar esta aba.", os avisos param, e a página tenta fechar a aba (os navegadores só deixam fechar abas abertas por script). Não usada, a página SHALL avisar e manter a edição para um novo envio.

#### Scenario: Aplicar na TV
- **WHEN** o usuário envia uma foto e pressiona Aplicar na TV
- **THEN** em até 2 s a página mostra "Pronto! O papel de parede está na TV." e "Você já pode fechar esta aba.", e para de avisar a TV

#### Scenario: Cancelar na TV
- **WHEN** o usuário envia uma foto e pressiona Cancelar na TV
- **THEN** a página mostra "A TV não usou esta foto. Ajuste e envie de novo." e a edição continua

#### Scenario: Aba aberta pela câmera
- **WHEN** a página foi aberta pelo QR e a TV aplica a foto
- **THEN** a aba continua aberta (o navegador não deixa fechá-la) com a mensagem de que pode ser fechada

### Requirement: Uma rota por função
Cada função da página SHALL ter o seu próprio caminho, com as suas rotas embaixo dele: o papel de parede usa `/wallpaper/` (a página) e `/wallpaper/upload` (o envio). As rotas da sessão (o aviso, `/ping`) e os recursos compartilhados (`/lib/`, como o decodificador HEIC) SHALL ficar fora dos caminhos das funções. Enquanto houver uma função só, `/` SHALL redirecionar para ela, mantendo o código.

#### Scenario: Endereço do papel de parede
- **WHEN** o usuário lê o QR da tela Receber imagem
- **THEN** o navegador abre `http://<ip>:<porta>/wallpaper/?c=<código>`

#### Scenario: Raiz digitada
- **WHEN** o usuário digita só `<ip>:<porta>` no navegador
- **THEN** o navegador é levado para `/wallpaper/`, que pede o código

#### Scenario: Caminho desconhecido
- **WHEN** o navegador pede `/fotos/`
- **THEN** a TV responde 404

### Requirement: Sem rede
Sem interface com IPv4 privado ativa, a tela Receber imagem SHALL NOT ligar o servidor nem mostrar QR. SHALL mostrar a mensagem de que a TV precisa estar na mesma rede que o celular. Ao conectar a rede com a tela aberta, SHALL passar a mostrar o QR sem o usuário sair.

#### Scenario: TV sem rede
- **WHEN** a TV não tem rede e o usuário abre a tela Receber imagem
- **THEN** a tela mostra "Conecte a TV à mesma rede do celular" no lugar do QR, e nenhuma porta é aberta

### Requirement: Página do Lume para o celular
O servidor SHALL servir uma página única, sem recursos externos (CSS, script e logo embutidos), com o logo e o nome "Lume" no topo e uma seção "Papel de parede": requisitos, escolher foto, edição sobre a prévia 16:9 (capacidade `custom-wallpaper`), enviar e progresso. Ela SHALL ser responsiva em smartphone, tablet e computador (ver "Layout responsivo da página") e SHALL funcionar nos navegadores atuais de Android, iOS, Windows, macOS e Linux.

#### Scenario: Abrir pelo QR
- **WHEN** o usuário lê o QR com a câmera do celular
- **THEN** o navegador abre a página com o logo, o nome "Lume" e a seção "Papel de parede", sem pedir o código, mostrando só a escolha da foto

#### Scenario: Requisitos visíveis
- **WHEN** a página é aberta
- **THEN** ela informa os formatos aceitos (JPEG, PNG, WebP, HEIC), o tamanho ideal (1920x1080 ou maior, na horizontal) e que a imagem é ajustada ali mesmo antes de ir para a TV

### Requirement: Layout responsivo da página
A página SHALL ter três arranjos por largura: smartphone (menos de 600 px), uma coluna com a prévia na largura toda e toque de pelo menos 48 px; tablet (600 a 1023 px), uma coluna centrada de até 640 px; computador (1024 px ou mais), duas colunas, com a prévia e os controles à direita. Em nenhum arranjo SHALL haver rolagem horizontal, e o texto SHALL ter pelo menos 16 px. Gestos sobre a prévia SHALL NOT rolar nem ampliar a página.

#### Scenario: Smartphone
- **WHEN** a página é aberta numa janela de 360 px de largura
- **THEN** tudo fica numa coluna, sem corte nem rolagem horizontal, e os botões têm pelo menos 48 px de altura

#### Scenario: Largura mínima
- **WHEN** a página é aberta numa janela de 320 px de largura
- **THEN** nada fica cortado e não há rolagem horizontal

#### Scenario: Tablet
- **WHEN** a página é aberta num tablet de 820 px de largura, em pé ou deitado
- **THEN** o conteúdo fica numa coluna centrada de até 640 px

#### Scenario: Computador
- **WHEN** a página é aberta num navegador de computador com 1440 px de largura
- **THEN** os requisitos e a escolha da foto ficam à esquerda e a prévia 16:9 com os controles à direita, lado a lado

#### Scenario: Girar o aparelho
- **WHEN** o usuário gira o celular ou o tablet com uma foto já escolhida e ajustada
- **THEN** a página se rearranja sem perder a foto, o ajuste nem o progresso

### Requirement: Escolha da foto no computador
No computador, além do botão de escolher arquivo, a página SHALL aceitar arrastar e soltar a imagem sobre a área de escolha, com destaque visual enquanto o arquivo está sobre ela.

#### Scenario: Arrastar e soltar
- **WHEN** o usuário arrasta um JPEG do explorador de arquivos e solta sobre a área de escolha
- **THEN** a página carrega a foto e abre a edição, como pelo botão

#### Scenario: HEIC no computador
- **WHEN** o usuário solta um HEIC no Chrome do computador
- **THEN** a página o decodifica pelo decodificador em WebAssembly e abre a edição, como com um JPEG

### Requirement: Idioma da página segue o celular
A página SHALL usar o idioma preferido do navegador do celular, com a mesma escolha de idioma da interface do Lume (variante exata, depois só idioma, depois inglês), e os mesmos arquivos de tradução.

#### Scenario: Celular em espanhol, TV em português
- **WHEN** o navegador do celular prefere espanhol e a TV está em português do Brasil
- **THEN** a página aparece em espanhol e a tela da TV continua em português

#### Scenario: Idioma sem tradução
- **WHEN** o navegador prefere japonês
- **THEN** a página aparece em inglês

### Requirement: Preparação da imagem no celular
A página SHALL decodificar a foto no próprio navegador, aplicar a rotação da foto e guardar só no aparelho uma cópia de trabalho reduzida por área até no máximo 4 MP, sobre a qual a edição acontece. Ao enviar, SHALL compor o resultado em 1920x1080 a partir dessa cópia e enviá-lo em QOI sem perda adicional. A página SHALL lembrar a foto e o ajuste enquanto continuar aberta, para o usuário enviar de novo com outro ajuste.

#### Scenario: Foto do iPhone em HEIC
- **WHEN** o usuário escolhe uma foto HEIC de 4032x3024 num iPhone
- **THEN** a página mostra a prévia na orientação certa e, ao enviar, a TV recebe uma imagem de 1920x1080

#### Scenario: Foto muito grande
- **WHEN** o usuário escolhe uma foto de 60 MP, ou um HEIC de 50 MP, num aparelho que não consegue abri-la inteira
- **THEN** a página a reduz no próprio aparelho (decodificando já reduzida, em etapas que cabem no navegador, e com uma cópia de trabalho menor se preciso) e abre a edição; só mostra erro se nem uma cópia de 1 MP der certo

#### Scenario: Imagem pequena demais
- **WHEN** o usuário escolhe uma imagem com o lado menor abaixo de 64 px
- **THEN** a página recusa a imagem com a mensagem do tamanho mínimo e não envia nada

#### Scenario: Formato não decodificável
- **WHEN** o navegador não consegue decodificar o arquivo escolhido
- **THEN** a página informa que o formato não é aceito e não envia nada

#### Scenario: Enviar de novo
- **WHEN** o usuário já enviou, muda o zoom na página e envia outra vez
- **THEN** a página envia o novo resultado sem pedir a foto de novo

### Requirement: HEIC em qualquer navegador
A página SHALL abrir fotos HEIC/HEIF também nos navegadores que não as decodificam (Chrome do Android e do computador), com um decodificador em WebAssembly servido pela própria TV, baixado só quando o navegador falha com um HEIC. A rotação gravada no arquivo SHALL ser aplicada. Se o decodificador também falhar, a página SHALL explicar como salvar a foto em JPEG.

#### Scenario: Foto HEIC no Android
- **WHEN** o usuário escolhe no Chrome do Android uma foto HEIC tirada pela câmera do aparelho
- **THEN** a página mostra "Preparando a imagem…", decodifica a foto na orientação certa e abre a edição

#### Scenario: Só quando preciso
- **WHEN** o usuário escolhe um JPEG, ou um HEIC no Safari do iPhone
- **THEN** a página não baixa o decodificador em WebAssembly

#### Scenario: Decodificador falha
- **WHEN** o arquivo HEIC está corrompido ou o aparelho não tem memória para decodificá-lo
- **THEN** a página informa que não conseguiu abrir a foto e sugere salvá-la em JPEG, sem enviar nada

### Requirement: Validação na TV
A TV SHALL tratar todo envio como não confiável. Corpo acima de 9 MB SHALL ser recusado antes de lido por inteiro. Cabeçalho QOI inválido, dimensões diferentes de 1920x1080 ou dados que não decodificam por inteiro SHALL ser recusados com erro claro à página, mantendo o servidor no ar para nova tentativa.

#### Scenario: Corpo grande demais
- **WHEN** chega um envio que declara 40 MB
- **THEN** a TV responde com erro de tamanho sem ler o corpo, e a página mostra "Imagem grande demais"

#### Scenario: Dados corrompidos
- **WHEN** chega um QOI com cabeçalho válido e dados truncados
- **THEN** a TV recusa, a página mostra "A TV não conseguiu ler a imagem" e o usuário pode tentar de novo

#### Scenario: Tamanho errado
- **WHEN** chega um QOI válido de 1280x720
- **THEN** a TV recusa com erro de leitura e continua esperando

#### Scenario: Envio válido
- **WHEN** chega um QOI válido de 1920x1080
- **THEN** a TV o aceita, abre a tela Confirmar, e a página mostra "Enviado. Confira na TV; você pode ajustar e enviar de novo"

### Requirement: Uma imagem por vez
O servidor SHALL atender um envio por vez. Enquanto uma imagem é recebida ou verificada, um segundo envio SHALL ser recusado com aviso de que a TV está ocupada. A tela da TV SHALL mostrar o estado (aguardando, celular conectado, recebendo com porcentagem, verificando), com no máximo 4 quadros por segundo de progresso.

#### Scenario: Dois celulares
- **WHEN** dois celulares enviam ao mesmo tempo
- **THEN** o primeiro é recebido, e o segundo recebe "A TV está recebendo outra imagem"

#### Scenario: Progresso na TV
- **WHEN** uma imagem de 8 MB está sendo recebida
- **THEN** a tela mostra "Recebendo" com a porcentagem, atualizada no máximo 4 vezes por segundo

### Requirement: Custo do servidor
O servidor SHALL usar uma única thread, que fica bloqueada à espera de conexão (sem laço acordado), e SHALL NOT pedir quadros ao laço, exceto para mostrar mudança de estado, progresso ou o relógio do último minuto. Um aviso da página que não muda o estado mostrado SHALL NOT pedir quadro. Com o servidor ligado e ocioso, o PSS SHALL ficar a até 2 MB do valor medido com a tela fechada.

#### Scenario: Tela aberta e ociosa
- **WHEN** a tela Receber imagem está aberta, fora do último minuto, sem nenhuma requisição
- **THEN** o Lume não desenha quadros, e a CPU do processo fica abaixo de 1% em média

#### Scenario: Avisos durante a edição
- **WHEN** a página avisa a TV a cada 30 s durante a edição
- **THEN** a TV só desenha quando o estado mostrado muda (por exemplo, ao passar a "Celular conectado · editando…")

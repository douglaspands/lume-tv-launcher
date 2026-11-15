## Purpose

Define a ferramenta Armazenamento: um teste curto da velocidade de escrita e leitura sequencial do armazenamento interno da TV, para explicar lentidão causada por memória flash lenta ou cheia.

## ADDED Requirements

### Requirement: Teste de velocidade do armazenamento
A ferramenta Armazenamento SHALL mostrar o espaço livre e total do armazenamento de dados e, só quando o usuário pressionar OK em "Testar", SHALL medir a escrita sequencial de um arquivo de 64 MB (gravado até o armazenamento confirmar) e a leitura sequencial do mesmo arquivo, sem que a leitura venha do cache de memória. Os resultados SHALL aparecer em MB/s. O teste SHALL rodar fora da thread de desenho e SHALL usar no máximo 1 MB de memória para os dados.

#### Scenario: Teste completo
- **WHEN** o usuário pressiona OK em "Testar" com espaço livre suficiente
- **THEN** a tela mostra a velocidade de escrita e a de leitura em MB/s

#### Scenario: Pouco espaço
- **WHEN** o espaço livre é menor que 256 MB
- **THEN** o teste não começa, e a tela explica que falta espaço

#### Scenario: Interface responsiva
- **WHEN** o teste está em andamento
- **THEN** a interface continua respondendo ao controle

### Requirement: Arquivo temporário do teste
O arquivo de teste SHALL ficar na pasta privada do Lume e SHALL ser apagado ao fim do teste, ao cancelar com Voltar e, se sobrar de uma execução interrompida, na próxima abertura da ferramenta.

#### Scenario: Fim do teste
- **WHEN** o teste termina
- **THEN** o arquivo de teste não existe mais

#### Scenario: Cancelar
- **WHEN** a escrita está em andamento e o usuário pressiona Voltar
- **THEN** o teste para, o arquivo é apagado, e a ferramenta continua aberta com "Teste cancelado"; um segundo Voltar fecha a ferramenta

#### Scenario: Arquivo esquecido
- **WHEN** o processo foi encerrado durante um teste e a ferramenta é aberta de novo
- **THEN** o arquivo de teste da execução anterior é apagado

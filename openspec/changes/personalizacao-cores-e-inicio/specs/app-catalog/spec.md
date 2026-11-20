## MODIFIED Requirements

### Requirement: Ícone do app no dock
Cada favorito do dock SHALL exibir a arte correspondente ao formato configurado em `"Formato dos favoritos"`: o ícone do app nos formatos `Redondo` (circular, padrão) e `Quadrado` (quadrado de cantos arredondados), ou o banner 16:9 de TV (o mesmo usado na grade de Apps, reutilizado do cache sem duplicar memória) no formato `Retangular`. Quando a imagem tem transparência, ela SHALL aparecer sobre a cor do app no formato configurado do dock, acompanhando o formato dos demais favoritos. Os ícones SHALL ser carregados preferencialmente na resolução mais alta disponível no aplicativo (com limite de tentativas e fallback limpo para a densidade padrão do sistema) e rasterizados com filtragem bilinear e anti-aliasing. As imagens SHALL ser carregadas aos poucos, sem atrasar o primeiro quadro nem a resposta às teclas, e a troca entre os formatos já em cache SHALL acontecer na hora, sem exigir releitura desnecessária. Enquanto a imagem não estiver disponível, o dock SHALL mostrar a cor do app com a inicial do nome. As imagens do dock SHALL ficar na mesma textura gráfica das imagens da grade de Apps, criada só com o lume em primeiro plano e liberada ao ir para segundo plano.

#### Scenario: App com banner de TV nos favoritos
- **WHEN** o Netflix, que declara banner de TV, é um favorito e o formato do dock está em `Redondo` ou `Quadrado`
- **THEN** o dock mostra o ícone do Netflix no formato configurado, e a grade de Apps continua mostrando o banner

#### Scenario: App com banner de TV nos favoritos (Retangular)
- **WHEN** o Netflix, que declara banner de TV, é um favorito e o formato do dock está em `Retangular`
- **THEN** o dock mostra o banner 16:9 do Netflix reutilizando a imagem da grade de Apps, sem alocar textura extra

#### Scenario: Ícones carregando
- **WHEN** o lume abre e os ícones ainda não foram carregados
- **THEN** o dock é desenhado de imediato com a cor e a inicial de cada app, e cada um dá lugar à imagem assim que ela fica pronta

#### Scenario: Segundo plano
- **WHEN** outro app passa ao primeiro plano
- **THEN** a textura dos ícones é liberada, e as imagens são carregadas de novo na próxima vez que o lume ficar visível

#### Scenario: Troca de formato sem recarregar ícones
- **WHEN** os ícones e banners já estão carregados e o usuário alterna entre `Redondo`, `Quadrado` e `Retangular`
- **THEN** o dock passa a exibir o novo formato imediatamente, sem voltar às iniciais

#### Scenario: Carregamento em alta resolução
- **WHEN** um app possui ícones em alta densidade (como `xxhdpi` ou superior)
- **THEN** o dock carrega o asset em alta resolução, rasterizando com filtragem suave e preservando a definição e nitidez

#### Scenario: Ícone com transparência acompanha os demais
- **WHEN** um app possui ícone com transparência ou formato não adaptativo (como o Globoplay)
- **THEN** ele é exibido sobre a base geométrica correspondente ao formato global do dock, acompanhando os demais aplicativos

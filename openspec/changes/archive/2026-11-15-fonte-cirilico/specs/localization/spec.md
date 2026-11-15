## MODIFIED Requirements

### Requirement: Nomes de apps em qualquer escrita
Nomes de apps e textos traduzidos SHALL ser exibidos mesmo quando usam caracteres fora da Plus Jakarta Sans embutida. O cirílico SHALL vir da Manrope embutida, no mesmo peso pedido para o texto (500, 700 ou 800), em qualquer aparelho e sem depender das fontes do sistema. As demais escritas (ex.: grego, chinês) SHALL recorrer às fontes do sistema. Escritas da direita para a esquerda e escritas que exigem shaping complexo SHALL exibir os caracteres na ordem lógica, sem garantia de forma correta.

#### Scenario: App com nome em chinês
- **WHEN** está instalado um app cujo nome é "爱奇艺"
- **THEN** o tile mostra os caracteres corretos, usando uma fonte do sistema

#### Scenario: Texto em russo com palavra em latim
- **WHEN** a interface está em russo e mostra "О Lume" no peso 700
- **THEN** "О" vem da Manrope 700 e "Lume" da Plus Jakarta Sans 700, sem usar a fonte do sistema

#### Scenario: Peso do cirílico
- **WHEN** um texto em cirílico é pedido no peso 800
- **THEN** os glifos vêm da Manrope 800, não de uma fonte regular do sistema

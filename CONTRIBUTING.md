# Contribuindo com o GIFTray

O GIFTray está no início da validação técnica. Contribuições são bem-vindas, especialmente relatos reproduzíveis de comportamento no Windows 10/11 e propostas para tornar a captura e o compartilhamento de GIFs mais simples.

## Antes de contribuir

Leia o [README](README.md), a [visão do produto](docs/product-vision.md) e os [critérios da P0](docs/p0-plan.md). Se quiser sugerir uma funcionalidade grande, discuta o problema primeiro: estamos evitando aumentar o escopo antes de validar a experiência central.

## Fluxo de trabalho

Adotamos `main` para versões estáveis e `develop` para integração. Crie uma branch pequena a partir de `develop` (`feature/`, `fix/`, `docs/` ou `test/`), use Conventional Commits e abra um pull request com contexto, passos de teste e limitações. Mudanças em APIs do Windows devem indicar o ambiente em que foram testadas.

Relatos de compatibilidade devem registrar versão do Windows e do aplicativo de origem/destino, tipos de dados observados e se GIFs permaneceram animados. Não envie imagens privadas, credenciais, URLs assinadas nem o conteúdo integral da área de transferência em issues.

## Licença e conteúdo

Ao contribuir com código neste repositório, você o oferece sob a [MPL-2.0](LICENSE) e declara ter direitos suficientes sobre sua contribuição. Não inclua GIFs ou imagens de terceiros sem permissão adequada. Nome e marca GIFTray não são licenciados como código-fonte.

A plataforma web comercial/privada, planejada para um repositório separado, não faz parte das contribuições a este repositório.

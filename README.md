# GIFTray

**O GIF certo, na hora certa.**

Você já perdeu o momento perfeito de mandar um GIF porque não conseguiu encontrá-lo entre dezenas de favoritos? O **GIFTray** é um aplicativo gratuito e open source para Windows 10 e 11 que pretende tornar duas ações mais fáceis: **salvar GIFs que você encontra** e **recuperá-los rapidamente enquanto conversa**.

> **Em desenvolvimento — sem versão pública disponível.** Este repositório contém a prova técnica inicial do aplicativo desktop. As funcionalidades a seguir representam nossa visão, não recursos já testados ou disponíveis.

## A experiência que queremos criar

**1. Salve sem interromper a conversa.** Arraste ou cole um GIF no aplicativo; quando a origem permitir, o GIFTray identifica a referência automaticamente. Quando houver apenas um arquivo, o plano é preservá-lo localmente. A organização pode esperar.

**2. Encontre em segundos.** Use um atalho global configurável para abrir uma busca compacta, digite o nome, uma tag ou um apelido e selecione o GIF sem precisar trocar de aplicativo manualmente.

**3. Compartilhe no seu ritmo.** Copie o link ou o arquivo animado quando o aplicativo de destino aceitar; baixar a mídia também será uma alternativa. A compatibilidade de arrastar e colar será medida nos testes, não presumida.

## O que torna o GIFTray diferente

- **Sua coleção, não outro catálogo:** os GIFs são escolhidos por você, de qualquer origem compatível. Não planejamos uma busca pública de GIFs no MVP.
- **Windows primeiro:** mesma funcionalidade essencial no Windows 10 22H2 e Windows 11, com visual inspirado nos controles nativos de cada sistema, temas automático/claro/escuro e atalho personalizável.
- **Busca local:** nomes, tags e coleções devem ser pesquisáveis sem depender de consultas ao servidor a cada tecla.
- **Salve primeiro, organize depois:** uma caixa de entrada recebe os itens sem obrigar o preenchimento de formulários.
- **Sem cobrança ou anúncios durante o uso rápido:** o foco é a experiência, não interromper a conversa.
- **Privacidade e custo reduzido:** usar URLs quando confiáveis, arquivos locais quando necessário e nenhuma hospedagem de mídias na nuvem por padrão.

## Estado atual

Estamos na **P0 — prova técnica**. O objetivo é validar atalho global, captura por arrastar/colar, preservação da animação ao copiar, foco da janela e integração visual com Windows 10/11. Nem todas as combinações de origem e aplicativo de destino permitirão o mesmo comportamento.

Depois da P0, construiremos a biblioteca desktop. Uma plataforma web privada para organização e sincronização está prevista para uma etapa posterior, em **outro repositório**, independente deste projeto open source.

Para acompanhar os critérios e as limitações, consulte a [visão do produto](docs/product-vision.md), a [arquitetura](docs/architecture.md), o [plano da P0](docs/p0-plan.md) e a [matriz de testes](docs/p0-test-matrix.md).

## Desenvolvimento e contribuições

Este repositório público abriga o **aplicativo desktop e, caso surja necessidade real, pacotes compartilhados do próprio desktop**. Ele usa um workspace `pnpm` e será desenvolvido com Tauri 2, React e TypeScript. As instruções para executar o aplicativo serão adicionadas quando o primeiro protótipo existir. Para fluxo de contribuição, consulte [CONTRIBUTING.md](CONTRIBUTING.md).

## Licença

O código deste repositório é disponibilizado sob a [Mozilla Public License 2.0](LICENSE) (MPL-2.0). A licença do código não concede direitos de uso da marca e da identidade visual GIFTray além dos estritamente previstos na própria licença. Imagens, GIFs ou outros conteúdos de terceiros mantêm seus respectivos direitos.

---

**GIFTray — Salve primeiro. Encontre na hora.**

# Visão de produto — GIFTray

> **Status:** visão e limites do MVP definidos; funcionalidades ainda não validadas em beta.  
> **Público deste documento:** produto, design, desenvolvimento, testes e pessoas interessadas em contribuir.

## Por que o GIFTray existe

Em uma conversa, a graça de uma reação muitas vezes depende do momento. Quem acumula dezenas ou centenas de GIFs costuma perder tempo navegando em favoritos desorganizados; salvar novas reações pode exigir tantos passos que o usuário desiste.

**Problema principal:** o GIF já foi visto e salvo em algum lugar, mas não é encontrado rápido o suficiente quando necessário.

**Proposta de valor:** **salve primeiro, organize depois; encontre o GIF certo na hora certa.** O GIFTray não pretende ser um catálogo de GIFs. Ele centraliza e organiza as referências escolhidas pelo usuário e facilita acessá-las independentemente do mensageiro.

## Para quem estamos construindo

Primeiro, pessoas que usam GIFs com frequência no Discord, WhatsApp Web, Telegram, fóruns ou outros aplicativos e já sentem dificuldade para gerenciar suas reações. Essa hipótese de público deve ser testada; não presumimos que todos irão abandonar seus favoritos atuais.

## Experiências centrais

### Cenário A — salvar enquanto conversa ou navega

A pessoa encontra um GIF, arrasta-o para a janela de captura ou cola com `Ctrl+V`. Se houver uma referência válida, o sistema a armazena. Se somente o arquivo estiver disponível, o aplicativo pode preservá-lo localmente. O item aparece em **Não organizados** sem exigir um nome ou tags naquele momento.

### Cenário B — encontrar e compartilhar

Com um atalho global configurável, a pessoa abre uma busca compacta sobre o aplicativo atual, digita um nome, apelido ou tag, seleciona o item e copia o link ou a mídia quando o destino permitir. O foco retorna à conversa. O sistema deve oferecer download como alternativa.

### Cenário C — organizar com calma

No desktop ou na futura biblioteca web, a pessoa revisa a caixa de entrada, renomeia, marca favoritos, adiciona tags e organiza em coleções. A organização é opcional na captura e progressiva ao longo do uso.

## Princípios de produto

| Princípio | Consequência prática |
|---|---|
| Velocidade antes de recursos | Pesquisa local no desktop, sem solicitar rede a cada tecla |
| Captura sem atrito | Arrastar, colar ou adicionar URL; nenhum formulário obrigatório para salvar |
| Compatibilidade honesta | Nunca prometer que qualquer app permite arrastar ou colar GIF animado |
| Windows 10/11 como prioridade | Mesmas funcionalidades essenciais; visual adaptado a cada versão e recursos disponíveis |
| Aparência familiar | Controles Fluent, tema seguindo Windows por padrão, atalho configurável, acessibilidade |
| Controle do usuário | Coleções privadas; leitura explícita de clipboard; exportação da biblioteca |
| Custo inicial mínimo | Sem mídia hospedada por padrão; serviços gratuitos apenas quando necessários |
| Independência de provedores | A biblioteca funciona sem KLIPY, GIPHY, Tenor ou modificações do Discord |

## O que compõe o MVP

**Desktop, Windows 10 22H2 e Windows 11:** janela rápida acionada por atalho; busca por nome/tags/coleções; navegação por teclado; tray; tema Sistema/Claro/Escuro; captura por arrastar/colar/URL; coleção local com Não organizados, favoritos e recentes; copiar link, copiar mídia quando tecnicamente possível e baixar.

**Web (repositório privado separado, futuro):** landing page para apresentar o produto e biblioteca autenticada para organizar referências e sincronizar metadados com o desktop. O usuário deve reconhecer quando um arquivo só existe em um computador.

**Tipos de item:** referência remota (URL válida), arquivo local (bytes persistidos no próprio PC) ou origem indisponível (metadados preservados e opção futura de reparo). Links assinados ou temporários não devem ser tratados como permanentes.

## Fora do MVP

Descoberta de GIFs por KLIPY/GIPHY/Tenor; importação automática de favoritos do Discord; mods; mobile; IA; biblioteca pública; envio automático em chats; mídia hospedada na nossa nuvem; anúncios e funcionalidades pagas.

## Como saberemos se há valor

Estas são metas experimentais, **não desempenho já medido**: captura mediana em até 3 segundos para origens compatíveis; busca até copiar em até 3 segundos em biblioteca local indexada; ao menos 80% de sucesso nas tarefas testadas; ao menos 50% dos participantes retornando após sete dias. Medir com transparência e sem telemetria invasiva por padrão.

## Questões em aberto

- Quais origens permitem drag-and-drop com URL utilizável? Testar, não presumir.
- Quais destinos aceitam colagem de arquivos GIF com animação preservada?
- Como comunicar itens locais em dispositivos onde o arquivo não existe?
- Quais limites de cache e fluxo de recuperação de links quebrados funcionam melhor?

Essas questões são o foco inicial da [prova técnica P0](p0-plan.md). A separação entre o desktop open source e a plataforma web privada está descrita em [Arquitetura](architecture.md).

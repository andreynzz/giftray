# Pesquisa — “Salvar no GIFTray” por menu de contexto

> **Escopo:** investigação arquitetural e de produto; não há extensão, integração Shell ou alteração de produção neste documento.
> **Base do projeto:** o GIFTray é um aplicativo desktop Windows. A P0 já valida entrada explícita, inspeção de mídia, download remoto protegido e clipboard; a P1 ainda precisa estabelecer o contrato de item persistente descrito na [auditoria arquitetural](architecture-audit.md).

## Resumo executivo

Uma opção de menu de contexto é tecnicamente viável em navegadores Chromium e no Explorador de Arquivos, mas são duas integrações externas com distribuição, permissões, instalação e manutenção próprias. Elas não devem ser usadas como atalho para pular a fundação da biblioteca local.

**Recomendação:** a P1 deve implementar somente o caso de uso seguro e persistente de importação que essas integrações consumirão depois: receber uma origem explícita, validar, copiar para armazenamento controlado quando necessário, criar o item da biblioteca e reportar o resultado. Drag-and-drop, colagem explicitamente armada e seleção de arquivo continuam os meios de captura de P1.

Após essa fundação, a primeira candidata é uma extensão Manifest V3 para Chrome/Edge que encaminhe uma URL pública HTTPS para o aplicativo por Native Messaging. O menu do Explorer pode ser uma etapa posterior para arquivos locais. A integração com o menu do Discord Desktop não é uma opção suportada para um aplicativo local; os fallbacks continuam sendo arrastar, salvar o arquivo ou colar explicitamente.

## Premissas e fronteiras

- “Salvar” significa criar um item durável da biblioteca, não colocar bytes no cache temporário de `CF_HDROP`.
- URL, caminho, HTML, mensagem da extensão e dados do clipboard são entradas não confiáveis. O processo Rust continua sendo a fronteira que valida formato, tamanho, destino e persistência.
- A política P0 para download remoto não deve ser enfraquecida: HTTPS/443, DNS público vinculado à conexão, redirects manuais, sem proxy automático, timeout e limite de 25 MB.
- A extensão nunca deve enviar cookies, cabeçalhos de autenticação, URL assinada a logs, nem converter-se em proxy livre para URLs arbitrárias.
- Não há promessa de capturar toda imagem visível. `blob:`, mídia autenticada, anti-hotlink, DRM e políticas do site podem impedir a recuperação do original.

## Comparação das alternativas

| Abordagem | Escopo de captura | Viabilidade | UX | Segurança/manutenção | Compatibilidade com P1 | Recomendação |
|---|---|---|---|---|---|---|
| Extensão MV3 + Native Messaging | Imagens em páginas Chrome/Edge com URL utilizável | Alta para URL pública direta; parcial para mídia protegida/blob | Boa: clique direito → confirmação/resultado | Média-alta: dois stores, IDs, host nativo e permissões | Boa, se enviar uma solicitação de importação, não bytes | Pós-fundação P1 |
| Extensão MV3 que baixa bytes no navegador | Imagens acessíveis pela extensão | Parcial | Pode parecer transparente, mas falha em credenciais/blob e é pesada | Alta: host permissions, CORS, mensagens grandes e superfície de dados | Fraca sem protocolo de streaming bem definido | Não na P1 inicial |
| Verbo/integração clássica do Explorer | Arquivos locais selecionados | Alta em Windows 10; aparece como legado em Windows 11 | Boa no Win10; degradada em Win11 | Média: registro, desinstalação, paths e testes Shell | Boa para importar um arquivo local | Futuro, se menu clássico for aceitável |
| `IExplorerCommand` moderno | Arquivos locais selecionados | Alta, porém exige DLL COM e identidade de pacote | Boa no menu moderno Win11 | Alta: DLL in-process, MSIX/sparse package e compatibilidade | Boa para importar um arquivo local | Pós-P1; não primeiro incremento |
| Menu de contexto Discord Desktop | Mensagens/imagens no cliente Discord | Não há integração oficial aplicável | Seria ótima, mas indisponível como extensão local | Risco alto se tentar automação/inspeção não oficial | Não aplicável | Não fazer |
| Drag/drop, salvar arquivo e colagem explícita | Arquivo/URL oferecido pelo usuário | Já comprovada como direção P0 | Menos imediata, mas previsível | Baixa e alinhada à privacidade | Essencial | P1 |

## 1. Chrome e Edge

### Capacidades disponíveis

Chrome suporta `chrome.contextMenus` em Manifest V3; a permissão `contextMenus` permite restringir uma entrada a `contexts: ["image"]`. O Edge Chromium declara compatibilidade com a API para menus de imagem, link e página. [Chrome contextMenus](https://developer.chrome.com/docs/extensions/reference/api/contextMenus) e [APIs suportadas pelo Edge](https://learn.microsoft.com/en-us/microsoft-edge/extensions/developer-guide/api-support) são as referências oficiais.

A extensão recebe metadados do clique, especialmente a URL da imagem quando o navegador a expõe. Isso não prova que a URL seja recuperável pelo processo nativo: ela pode ser um `blob:`, expirar, exigir cookies, exigir `Referer` ou apontar para uma miniatura. A extensão pode usar script de conteúdo para obter contexto adicional do DOM, mas ele executa em mundo isolado; código e mensagens vindos da página devem ser tratados como não confiáveis. [Content scripts](https://developer.chrome.com/docs/extensions/develop/concepts/content-scripts).

Native Messaging é a ponte oficial para um aplicativo instalado. O service worker da extensão usa `runtime.sendNativeMessage()` ou `runtime.connectNative()`; o host se comunica por `stdin`/`stdout` e exige a permissão `nativeMessaging`. Scripts de conteúdo não podem chamar o host diretamente. O próprio Chrome orienta validar `sender.origin`/`sender.url` e sanitizar a mensagem antes de encaminhá-la. [Native Messaging — Chrome](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging).

No Windows, o instalador registra o manifesto do host por navegador no Registro. O Edge documenta chaves por usuário ou máquina, `allowed_origins` com os IDs exatos da extensão e observa que os IDs do Chrome Web Store e Edge Add-ons podem ser diferentes. O protocolo limita mensagens do host para o Edge a 1 MB; portanto, não é apropriado devolver ou transportar mídia inteira por uma única resposta. [Native Messaging — Edge](https://learn.microsoft.com/en-us/microsoft-edge/extensions/developer-guide/native-messaging).

### Arquitetura mínima para navegador

```text
Clique direito em imagem
        │
Extensão MV3: item “Salvar no GIFTray” (contexto image)
        │  URL + origem da página + metadados mínimos, sem cookies/bytes
        ▼
Service worker valida o contexto e envia mensagem versionada
        ▼
Native Messaging host pequeno, permitido apenas aos IDs publicados
        │  encaminha para o GIFTray já instalado
        ▼
Comando Rust de importação persistente
        │  revalida URL e executa a política remota existente
        ▼
Arquivo controlado + item da biblioteca + resultado sanitizado
```

O host nativo não deve duplicar downloader, banco nem regras de mídia. Sua responsabilidade futura é autenticar a origem da extensão, limitar o tamanho da mensagem, abrir/encaminhar a solicitação ao GIFTray e devolver um status genérico. Se o aplicativo não estiver disponível, o resultado deve explicar que o GIFTray precisa estar instalado/aberto conforme o protocolo escolhido, sem expor caminho local ou URL sensível.

### Segurança, permissões e falhas esperadas

1. Usar somente `contextMenus` e `nativeMessaging` no início. `activeTab` pode ser considerado se houver necessidade objetiva de contexto da aba. Não pedir `cookies`, `webRequest`, `tabs` amplo, nem host permission global por conveniência.
2. Para um download no service worker, o Manifest V3 exige `host_permissions`; permissões de host amplas são uma superfície relevante e a documentação alerta para não permitir que scripts de conteúdo solicitem qualquer URL arbitrária. [Cross-origin requests](https://developer.chrome.com/docs/extensions/develop/concepts/network-requests).
3. A primeira versão não deve baixar bytes no browser. Ela envia uma URL pública HTTPS ao fluxo Rust P0. Isso evita transportar mídia grande pelo Native Messaging e evita propagar credenciais do navegador.
4. `blob:` não é uma URL de rede reutilizável pelo host nativo. O comportamento depende do contexto/origem que criou o blob e não deve ser prometido. A UI deve informar “esta imagem não tem uma origem recuperável” e oferecer drag-and-drop, salvar o arquivo ou colagem explícita.
5. CORS, cookies ou permissões da extensão não tornam um arquivo protegido um recurso seguro para o GIFTray baixar. Mesmo quando a extensão consegue ler algo, reutilizar cookies/sessão fora do navegador amplia a exposição de credenciais e não é aceitável para a primeira versão.
6. O aplicativo revalida toda mensagem: versão do protocolo, ID/origem declarada pelo host, URL HTTPS sem credenciais/IP literal, teto de tamanho e assinatura final. A extensão não é uma autoridade de segurança.

### Instalação e distribuição

- Chrome: extensão MV3 empacotada e submetida ao Chrome Web Store; a publicação exige dashboard e revisão. Permissões amplas podem ampliar o escrutínio. [Publicar no Chrome Web Store](https://developer.chrome.com/docs/webstore/publish) e [processo de revisão](https://developer.chrome.com/docs/webstore/review-process).
- Edge: a mesma base Chromium tende a portar com poucas mudanças, mas deve ser testada e publicada no Edge Add-ons/Partner Center; o ID publicado precisa constar em `allowed_origins`. [Portar extensão Chrome para Edge](https://learn.microsoft.com/en-us/microsoft-edge/extensions-chromium/developer-guide/port-chrome-extension) e [publicar no Edge Add-ons](https://learn.microsoft.com/en-us/microsoft-edge/extensions/publish/publish-extension).
- O instalador do desktop precisa instalar/remover o manifesto do host e a chave de registro correspondente, por usuário quando possível. A desinstalação deve remover somente os artefatos que criou e nunca apagar a extensão do navegador sem consentimento explícito.
- Há dois ciclos de release e suporte: desktop/host nativo e extensão/store. Atualizações incompatíveis exigem versionamento de protocolo e compatibilidade temporária.

### Avaliação

| Critério | Avaliação |
|---|---|
| Viabilidade técnica | Alta para URL pública direta; parcial em conteúdo com sessão, `blob:` ou proteção de origem. |
| Experiência | Boa para casos suportados, desde que informe claramente o fallback quando não houver URL recuperável. |
| Dependências | Manifest V3, Chrome Web Store, Edge Add-ons, Native Messaging host e instalador Windows. |
| Complexidade | Média-alta; é outro produto distribuído, não apenas um botão do Tauri. |
| Manutenção | Média-alta por políticas de stores, IDs por store e mudanças de APIs Chromium. |
| Compatibilidade P1 | Boa depois que houver um caso de uso de importação persistente estável. |

## 2. Windows 10 e Windows 11 — Explorador de Arquivos

### Opções de integração

**Verbo/handler legado.** O Shell permite handlers COM `IContextMenu`, registrados para tipos de arquivo/objetos do Shell; handlers são DLLs in-process e requerem registro COM. É a rota tradicional do menu de contexto, aplicável a Windows 10, mas no Windows 11 o menu clássico fica em “Mostrar mais opções”. [Criar Shell Extension Handlers](https://learn.microsoft.com/en-us/windows/win32/shell/handlers) e [IContextMenu](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nn-shobjidl_core-icontextmenu).

**Comando moderno do Windows 11.** Para aparecer no menu moderno, a Microsoft orienta `IExplorerCommand` em DLL COM nativa, registrado em `windows.comServer` e `windows.fileExplorerContextMenus`. O aplicativo precisa de identidade de pacote; um Win32 não empacotado pode usar sparse package. A DLL é carregada pelo Explorer: construir o menu deve ser rápido e o trabalho demorado só começa após `Invoke`. [Comando de contexto no File Explorer](https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/integrate-packaged-app-with-file-explorer).

Um Tauri padrão não fornece automaticamente essa DLL COM nem o pacote/sparse package. Isso adiciona uma tecnologia nativa complementar ao processo Rust/Tauri, além de requisitos de arquitetura da DLL compatível com o Explorer e testes de instalação. Não é uma pequena configuração de `tauri.conf.json`.

### Fluxo recomendado, quando esta integração existir

```text
Arquivo(s) selecionado(s) no Explorer
        │
Handler/command Shell rápido: filtra extensão e obtém seleção
        │
        └─ não lê bytes, não faz rede, não bloqueia o Explorer
        ▼
Inicia/aciona o GIFTray com seleção delimitada
        ▼
Importação Rust: resolve caminho, rejeita fora do escopo/erros,
copia bytes para diretório controlado, confirma assinatura/tamanho
        ▼
Item persistente e feedback ao usuário
```

O caminho recebido do Shell é entrada não confiável. A importação precisa abrir e validar o arquivo, lidar com arquivo removido/trocado entre seleção e leitura, não seguir uma suposição baseada somente na extensão e gravar uma cópia no diretório da biblioteca. Não se deve fazer o Explorer depender do cache temporário de clipboard.

### Instalação, desinstalação e diferenças de versão

| Tema | Windows 10 | Windows 11 |
|---|---|---|
| Menu recomendado | Handler/verbo clássico pode ser apresentado no menu tradicional. | `IExplorerCommand` + identidade de pacote para o menu moderno. |
| Alternativa | Mesmo handler/registro clássico. | Handler clássico pode aparecer em “Mostrar mais opções”; não atende ao objetivo de presença no primeiro nível moderno. |
| Empacotamento | Instalador deve registrar e remover COM/verbo de forma reversível. | MSIX ou sparse package registra a extensão moderna; a Microsoft documenta sparse package para Win32 não empacotado. |
| Atualização | Atualizar DLL e registro sem deixar CLSID órfão; pode ser necessário reiniciar Explorer para recarregar registro. | Atualizar pacote/DLL compatível com a arquitetura do Explorer; pode ser necessário reiniciar Explorer ou encerrar sessão para o comando aparecer. |
| Desinstalação | Exige limpeza explícita e segura de registro/arquivos pertencentes ao produto. | Pacotes MSIX oferecem rastreamento/remoção limpa dos artefatos do pacote, mas dados criados pelo usuário exigem política própria. [Visão de contêiner MSIX](https://learn.microsoft.com/en-us/windows/msix/msix-containerization-overview) |

### Avaliação

| Critério | Verbo/legado | `IExplorerCommand` moderno |
|---|---|---|
| Viabilidade técnica | Alta | Alta, com investimento nativo específico |
| UX Windows 10 | Boa | Pode exigir uma estratégia adicional de compatibilidade |
| UX Windows 11 | Degradada para menu clássico | Boa no menu moderno |
| Dependências | Registro e possível DLL/launcher | DLL COM nativa, MSIX/sparse package, manifesto e instalador |
| Complexidade/manutenção | Média | Alta |
| Compatibilidade P1 | Boa para importar arquivos persistentes | Boa, mas prematura antes de estabilizar a importação |

## 3. Aplicativos externos — Discord Desktop

As Application Commands documentadas pelo Discord são comandos de aplicações/bots no cliente (chat, mensagem ou usuário) e são criadas pela API do Discord para um `application_id`; elas não são uma API para um programa desktop instalar uma opção local no menu de imagens do cliente Discord. [Application Commands — Discord](https://docs.discord.com/developers/interactions/application-commands).

Assim, não há uma integração oficial documentada que permita ao GIFTray inserir “Salvar no GIFTray” no menu de mídia do Discord Desktop. Tratar a ausência dessa superfície como convite para injeção, automação da UI, leitura contínua de clipboard ou inspeção de tráfego seria frágil, invasivo, incompatível com o princípio de entrada explícita e criaria custo de manutenção desproporcional.

Alternativas compatíveis:

- arrastar a mídia para o GIFTray quando o Discord/Windows fornecer arquivo ou URL utilizável;
- usar uma ação explícita do usuário para copiar/colar ou salvar o arquivo e então importá-lo;
- para Discord Web, uma extensão Chromium futura pode funcionar apenas no contexto da página e sob as mesmas limitações de URL, cookies e conteúdo protegido descritas acima.

## O que entra na P1 e o que fica para depois

### P1 — incluir

1. Contrato de item persistente e caso de uso de importação, conforme a auditoria: arquivo local, URL remota e origem indisponível são estados distintos.
2. Diretório de biblioteca controlado e operações transacionais entre arquivo e metadados; o cache `clipboard-output` continua temporário e separado.
3. Validação de assinatura/tamanho e feedback seguro no caminho de importação persistente.
4. Captura explícita já disponível (arquivo selecionado, drag-and-drop e colagem armada), com UX de produto em vez de tela diagnóstica.
5. Um contrato interno mínimo de solicitação de importação, versionado e com origem declarada, que uma extensão/Explorer poderá consumir depois. Isso é um contrato de caso de uso, não um framework de plugins.

### Pós-P1 — avaliar em incrementos separados

1. **Extensão Chrome/Edge:** protótipo com um único menu para imagens e Native Messaging que envie somente URL pública. Testar distribuição e IDs antes de prometer suporte geral.
2. **Explorer clássico:** decidir se “Mostrar mais opções” no Windows 11 é aceitável para uma primeira integração de arquivo local. Se sim, implementar/instalar/desinstalar o caminho mais simples em incremento separado.
3. **Explorer moderno Windows 11:** somente se a evidência de uso justificar DLL COM + MSIX/sparse package e uma matriz de testes de instalação/atualização.
4. **Conteúdo autenticado/blob:** pesquisar somente após os casos públicos terem valor comprovado. Não transferir cookies ou credenciais para o GIFTray como extensão da P1.

### Não incluir

- Extensão que tenta capturar todas as imagens, blobs ou recursos com sessão.
- Automação, injeção não oficial ou leitura contínua de dados do Discord Desktop.
- Downloader alternativo que contorne a política SSRF/DNS/TLS já auditada.
- Sistema genérico de plugins ou serviço em background aguardando capturas.

## Riscos e limitações a comunicar ao usuário

- Uma entrada de menu no navegador confirma a intenção do usuário, não a disponibilidade do original animado.
- Mídia que aparece animada pode ser uma miniatura, uma URL temporária ou um `blob:` não reutilizável.
- A extensão e o host nativo acrescentam autorização, distribuição e suporte independentes do desktop; atualização de um pode quebrar o outro sem protocolo versionado.
- Shell extensions rodam na experiência do Explorer; lentidão ou falha em DLL in-process afeta uma superfície crítica do Windows. O código de menu deve ficar mínimo.
- A presença e a posição do menu precisam ser validadas manualmente em Windows 10 22H2 e Windows 11; não são demonstradas por testes unitários.
- A compatibilidade de animação no aplicativo de destino continua sendo responsabilidade do destino, como já registrado na matriz P0.

## Matriz mínima de validação futura

| Área | Cenários manuais necessários |
|---|---|
| Chrome/Edge | GIF/WebP/PNG público; redirect; URL expirada; `blob:`; imagem com sessão; extensão sem host; host com ID não permitido; atualização/desinstalação do desktop. |
| Privacidade | Confirmar que logs não contêm URL assinada, cookies, caminho privado ou bytes; revisar permissões exibidas pelo store. |
| Explorer Win10 | GIF/WebP/PNG; múltipla seleção; caminho com Unicode; arquivo removido antes da importação; atualização e desinstalação. |
| Explorer Win11 | Presença no menu moderno ou em “Mostrar mais opções” conforme decisão; reinício do Explorer; MSIX/sparse package; 64 bits. |
| Discord | Drag/drop, salvar arquivo, colagem explícita e Discord Web com extensão, sem afirmar suporte ao cliente desktop. |

## Fontes oficiais consultadas

- [Chrome — Context Menus API](https://developer.chrome.com/docs/extensions/reference/api/contextMenus)
- [Chrome — Native Messaging](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging)
- [Chrome — Cross-origin network requests](https://developer.chrome.com/docs/extensions/develop/concepts/network-requests)
- [Chrome — Content scripts](https://developer.chrome.com/docs/extensions/develop/concepts/content-scripts)
- [Chrome Web Store — publicação](https://developer.chrome.com/docs/webstore/publish) e [revisão](https://developer.chrome.com/docs/webstore/review-process)
- [Microsoft Edge — APIs suportadas](https://learn.microsoft.com/en-us/microsoft-edge/extensions/developer-guide/api-support)
- [Microsoft Edge — Native Messaging](https://learn.microsoft.com/en-us/microsoft-edge/extensions/developer-guide/native-messaging)
- [Microsoft Edge — publicação](https://learn.microsoft.com/en-us/microsoft-edge/extensions/publish/publish-extension)
- [Microsoft — comando de contexto moderno no File Explorer](https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/integrate-packaged-app-with-file-explorer)
- [Microsoft — Shell Extension Handlers](https://learn.microsoft.com/en-us/windows/win32/shell/handlers)
- [Microsoft — IContextMenu](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nn-shobjidl_core-icontextmenu)
- [Discord — Application Commands](https://docs.discord.com/developers/interactions/application-commands)

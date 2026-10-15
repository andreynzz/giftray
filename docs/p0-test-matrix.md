# P0 — Matriz de testes de interoperabilidade

> **Status atual: não executado.** Esta página registra **observações verificáveis**, não expectativas, estimativas ou o que uma API promete na documentação.

## Como ler e preencher

Para cada cenário, identifique o ambiente (Windows 10 22H2 ou Windows 11, builds, aplicativo e versão), a mídia usada e os passos exatos. Registre se a importação forneceu uma URL, HTML, arquivo ou nenhum dado; depois confira se a animação continua presente no destino. **Não testado** nunca significa **Passou**.

Estados permitidos: **Não testado**, **Passou**, **Parcial**, **Falhou**, **Bloqueado**. Em cenários parciais ou com falhas, descreva o fallback apresentado ao usuário. Evite registrar dados sensíveis, conteúdo integral do clipboard e URLs com tokens.

## Ambiente de execução

| Campo | Registro |
|---|---|
| Sistema operacional e build | Pendente: testar Windows 10 22H2 e Windows 11 |
| Computador físico ou máquina virtual | Pendente |
| Versão do protótipo / Tauri | Pendente |
| Aplicativos externos e versões | Pendente |
| GIF de teste (formato, quadros, duração, tamanho) | Pendente |
| Data e responsável | Pendente |

## Importação: origem → GIFTray

| Origem | Entrada | Dados recebidos | Referência durável? | Animação preservada? | Resultado |
|---|---|---|---|---|---|
| Navegador — GIF direto | Arrastar | — | — | — | Não testado |
| Navegador — página com GIF | Arrastar | — | — | — | Não testado |
| Discord Web — mensagem | Arrastar | — | — | — | Não testado |
| Discord Desktop — mensagem | Arrastar | — | — | — | Não testado |
| Discord Desktop — favoritos | Arrastar | — | — | — | Não testado |
| Explorador de Arquivos — `.gif` | Arrastar | — | Não aplicável | — | Não testado |
| Área de transferência — URL | Colar explicitamente | — | — | — | Não testado |
| Área de transferência — mídia/arquivo | Colar explicitamente | — | — | — | Não testado |

### Registro P0.1 — mecanismo de diagnóstico

| Cenário | Observação | Resultado | Limitação/fallback |
|---|---|---|---|
| Implementação anterior, janela Tauri padrão | `dragDropEnabled` não era declarado; no Windows o padrão nativo do Tauri captura drops de arquivo antes dos handlers HTML5. Como o app só possuía handlers React, nenhum mecanismo nativo era observado. | Falhou | Corrigido com dois perfis mutuamente exclusivos: HTML5 padrão (`false`) e nativo experimental (`true`). Ainda requer teste por origem no Windows. |
| Implementação anterior, `Ctrl+V` | O listener existia somente na área de drop, mas a busca recebe foco ao abrir. Logo, a colagem não chegava ao handler. A primeira correção ainda focava um `div` não editável e descartava alvos editáveis. | Falhou | Corrigido por uma `textarea` dedicada: o botão arma e foca o destino, e o listener de `document` aceita somente esse alvo. |
| Perfil HTML5, `dragDropEnabled: false` | Evidência manual em 2026-10-13, origem não registrada: `drop`, `files`, 1 arquivo; classificado como `arquivo` e `imagem`. | Parcial | Confirma a entrega HTML5 para arquivo/imagem; ainda testar URL e HTML por origem. Não recebe eventos nativos de caminho do Tauri na mesma WebView. |
| Perfil Tauri, `dragDropEnabled: true` | Deve fornecer `onDragDropEvent` com contagem de caminhos de arquivo em `enter`/`drop`. | Não testado | Não fornece MIME, URL ou HTML; caminhos completos não são exibidos. |
| Colagem armada na área de diagnóstico | Evidência anterior em 2026-10-13: `paste (document)`, `text/plain`, 0 arquivos; o alvo não era editável. A P0.1 revisada usa `textarea` dedicada e mostra `paste (document → textarea)`. | Parcial | Repetir `Copy Media Link` para URLs `.gif` e `.webp`; a expectativa inicial é `text/plain`, 0 arquivos. A colagem em busca e outros campos editáveis segue o comportamento normal e não entra no diagnóstico. |

**Passos para os perfis de drag-and-drop:** executar `pnpm dev` para HTML5 e `pnpm --filter @giftray/desktop tauri:dev:native-dnd` para o perfil nativo. Em cada um, testar arrastar URL, página HTML e arquivo/imagem; registrar somente os tipos mostrados, a contagem e o resultado, nunca conteúdo, caminhos completos ou URLs assinadas.

**Passos para colagem P0.1 revisada:** execute `pnpm dev`, clique em **Ativar colagem de diagnóstico** e confirme que o foco vai para a textarea. Cole uma URL obtida por “Copy Media Link” de um `.gif` e depois de um `.webp`. Resultado esperado: `clipboard`, `paste (document → textarea)`, `text/plain`, 0 arquivos, sem mostrar o valor. Se a mensagem continuar “Captura armada”, o evento não foi disparado; se mostrar falha sem MIME e sem arquivos, o evento foi disparado sem dados utilizáveis.

### Registro P0.2 — importação e prévia temporária

| Cenário | Observação | Resultado | Limitação/fallback |
|---|---|---|---|
| Validação unitária de normalização, assinatura e seleção | 23 testes automatizados verificam URL HTTPS, `text/plain`, extração segura de atributos HTML, query/fragmento, assinaturas GIF/WebP/PNG, estados de animação, seleção entre miniatura e URL, estados técnicos e liberação de object URL. | Passou | Não equivale a interoperabilidade no WebView2 ou com apps externos. |
| URL colada não carregava na prévia | A URL HTTPS era recebida, normalizada e selecionada; a CSP continha `img-src 'self' asset: data:` e bloqueava a imagem remota. | Passou após a correção, conforme evidência manual informada abaixo | `https:` foi adicionado somente a `img-src`; não generaliza o resultado para todas as origens remotas. |
| Prévia de arquivo/URL no Windows | A implementação usa assinatura local e `<img>` nativo; URL remota não é baixada por proxy. | Não testado | Requer verificação real de GIF/WebP animados, arquivo PNG, URL expirada e CORS. |

### Evidência manual informada — P0.2

Os resultados abaixo foram informados após validação manual. O ambiente, as versões dos aplicativos e a versão do Windows não foram registrados; portanto, eles comprovam os fluxos exercitados, mas não compatibilidade universal nem cobertura específica de Windows 10/11.

| Cenário | Resultado informado | Limitação/fallback |
|---|---|---|
| Discord — arrastar mídia | Passou: importação e prévia funcionaram. | Não comprova todos os tipos de mensagem, favoritos ou versões do Discord. |
| URL HTTPS de GIF animado — colagem explícita | Passou: a prévia remota carregou e preservou a animação. | Depende da disponibilidade da origem e de aceitar requisição sem referer. |
| URL HTTPS de WebP animado — colagem explícita | Passou: a prévia remota carregou e preservou a animação. | Não prova suporte a todos os WebPs nem a URLs sem extensão reconhecível. |
| Substituição e limpeza de prévias | Passou. | Continua sem persistência entre sessões. |
| URL inválida | Passou: entrada inválida recebeu tratamento de erro. | Não representa todos os modos de expiração, bloqueio de hotlink ou CORS. |
| Busca, atalho global e Ctrl+V | Passou sem regressões observadas. | Atalho e comportamento visual ainda não foram registrados separadamente em Windows 10 22H2 e Windows 11. |

**Passos para importação P0.2:** execute `pnpm dev`, arraste para a zona HTML5 um GIF, WebP ou PNG local de até 25 MB e confirme a prévia, a origem local, o formato confirmado e a limpeza. Depois, arme a colagem e cole uma URL HTTPS `.gif` e uma `.webp`, incluindo uma variante com query string e fragmento; confirme que a origem aparece como remota e que o formato/animação são inferidos, sem que a URL integral seja exibida. Confirme a reprodução quando a origem permitir requisição sem referer. Repita com URL expirada ou bloqueada para confirmar o estado de erro. Teste separadamente navegador e Discord; registre apenas o mecanismo, formato, resultado e evidência sanitizada.

## Compartilhamento: GIFTray → destino

| Destino | Copiar URL | Copiar mídia/arquivo | Animação preservada? | Resultado |
|---|---|---|---|---|
| Discord Desktop | — | — | — | Não testado |
| Discord Web | — | — | — | Não testado |
| WhatsApp Web | Não testado | Aceita colagem | Não | Parcial |
| Telegram Desktop/Web | — | — | — | Não testado |
| Explorador de Arquivos (colar arquivo) | Não aplicável | — | — | Não testado |

### Registro P0.3 — clipboard de saída

| Cenário | Observação automatizada | Resultado | Limitação/fallback |
|---|---|---|---|
| Seleção da ação | Testes de unidade habilitam **Copiar link** somente para a URL HTTPS remota original e **Copiar arquivo** somente para arquivo local importado. | Passou | Não prova escrita real no clipboard. |
| Validação do arquivo | O comando nativo limita a 25 MB e confirma assinatura/extensão GIF, WebP ou PNG antes de armazenar os bytes. | Pendente de execução manual nativa | A validação de assinatura do frontend já tem cobertura P0.2; a ponte Tauri ainda precisa ser exercitada na janela Windows. |
| Retenção de cache | Política implementada: 24 h, máximo 100 MB, limpeza na inicialização e antes de nova cópia. | Pendente de execução manual nativa | Não remover o arquivo logo após copiar ou fechar a janela. |
| Download remoto | Deliberadamente não implementado. | Não aplicável | O usuário pode copiar o link HTTPS; não há download sem controles completos de SSRF/redirecionamento/DNS. |
| WhatsApp Web — GIF por arquivo | O destino aceitou a colagem do GIF, mas apresentou a imagem estática. Como controle, o mesmo GIF copiado diretamente do Windows Explorer também ficou estático. | Parcial | A perda de animação não é atribuída exclusivamente ao GIFTray/`CF_HDROP`; indica tratamento do destino. Não alterar esta integração para tentar compensar esse comportamento. Métodos alternativos de envio podem ser investigados em experimento futuro. |
| Download remoto P0.4 | Implementado com TLS, DNS público vinculado à conexão, sem proxy/redirecionamento automático, limite de 25 MB e verificação de assinatura. | Não testado manualmente | Ainda testar GIF/WebP/PNG, redirecionamento, timeout, servidor que retorna HTML e origem autenticada sem registrar URLs assinadas. |
| KLIPY GIF remoto → Discord Desktop | Aprovado manualmente: o GIF remoto foi copiado como arquivo e aceito pelo destino. | Passou | Ambiente e versões ainda não registrados por Windows. |
| fxtwitter WebP remoto → Discord Desktop | Aprovado manualmente: o WebP remoto foi copiado como arquivo e aceito pelo destino. | Passou | Não generaliza para todos os WebPs ou servidores. |
| GIF remoto → Explorer | Aprovado manualmente: o arquivo remoto foi colado no Explorer. | Passou | Ainda verificar nome, bytes e retenção em ambiente registrado. |
| GIF remoto → Discord Web | Aprovado manualmente: o destino aceitou a colagem. | Passou | Requer repetir com versões e ambiente registrados. |
| URL expirada ou bloqueada | Aprovado manualmente: a falha foi comunicada sem indicar cópia bem-sucedida. | Passou | Não cobre todos os erros de transporte. |
| GIF local → Discord | Aprovado manualmente sem regressão. | Passou | Mantém a limitação de compatibilidade já registrada para outros destinos. |
| Clipboard após fechar GIFTray | Aprovado manualmente: o arquivo permaneceu disponível após fechar o aplicativo. | Passou | Ainda não prova proteção contra limpeza por teto de cache. |
| Cache sob limite com arquivo CF_HDROP protegido | Teste Rust cria cache controlado com arquivo ativo e descartável: remove o descartável, preserva o ativo e recusa limpeza quando apenas o ativo excede o limite. | Passou (teste determinístico) | A leitura real do CF_HDROP e falhas Win32 continuam dependentes de validação manual Windows. |
| Processamento HTTP remoto | Testes Rust simulam cabeçalhos e chunks no mesmo processador usado pelo `reqwest`: 200, 404, 500, redirect HTTPS/proibido, `Content-Length` ausente/acima do limite, erro de leitura, tamanho excedido e preservação de bytes. | Passou (transporte simulado) | Handshake TLS e resolução DNS reais requerem teste de integração controlado. |
| Orquestrador do downloader remoto | Testes Rust com transporte simulado exercitam `download_with`, o mesmo fluxo de `ReqwestTransport`: sucesso com chunks de GIF e bytes exatos, 404/500, erro de requisição/leitura sem retorno parcial, redirect HTTPS, três redirects, quarto e loop rejeitados, destinos HTTP/IP privado rejeitados, `Content-Length` ausente/enganoso e teto efetivo de 25 MB. O histórico das URLs solicitadas é verificado. | Passou (transporte simulado) | Não substitui integração de rede real, TLS, DNS ou Windows. |
| Timeout global do download remoto | Transporte simulado com atraso controlado excede limite curto interno e `download_with_timeout` falha; produção usa o mesmo wrapper com 30 s para DNS, requisições, redirects e streaming. | Passou (transporte simulado) | Não mede latência nem handshake de servidores reais. |

**Passos P0.3 para validação manual:** importe separadamente um GIF animado, WebP animado e PNG local de até 25 MB. Clique em **Copiar arquivo**, cole no Explorador de Arquivos e verifique nome, extensão e bytes; em seguida cole manualmente no Discord Desktop, Discord Web, WhatsApp Web e Telegram Desktop, registrando aceitação, arquivo recebido e animação. Para URL HTTPS, clique em **Copiar link** e cole no Bloco de Notas; confirme que o valor é o original, inclusive query/fragmento, sem registrá-lo nesta matriz. Execute esses testes em Windows 10 22H2 e Windows 11 antes de alterar os estados gerais de compatibilidade.

## Comportamento do aplicativo

| Cenário | Windows 10 22H2 | Windows 11 |
|---|---|---|
| Registro e conflito do atalho global | Não testado | Não testado |
| Troca de atalho e persistência após reinício | Não testado | Não testado |
| Abertura, foco da busca e fechamento com Esc | Não testado | Não testado |
| Retorno de foco ao aplicativo anterior | Não testado | Não testado |
| Tema seguindo Windows enquanto o app está aberto | Não testado | Não testado |
| Tema Claro/Escuro manual e fallback visual | Não testado | Não testado |
| Alto contraste e preferência por movimento reduzido | Não testado | Não testado |
| DPI elevado e múltiplos monitores | Não testado | Não testado |
| Funcionamento com WebView2 disponível/ausente | Não testado | Não testado |

## Ficha de evidência reproduzível

> Duplicar esta ficha para cada teste relevante.

**Cenário:**  
**Data, responsável e ambiente:**  
**Aplicativo de origem/destino e versão:**  
**Arquivo utilizado e características:**  
**Passos:**  
**Esperado:**  
**Observado:**  
**Resultado:** Não testado / Passou / Parcial / Falhou / Bloqueado  
**Evidência:** captura de tela, vídeo local, testes automatizados ou logs sanitizados  
**Limitação e fallback:**

## Conclusão da P0

**Decisão:** pendente.  
**Cenários comprovadamente suportados:** pendente.  
**Riscos remanescentes:** pendente.  
**Próximo incremento autorizado:** pendente.

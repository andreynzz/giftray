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
| URL colada não carregava na prévia | A URL HTTPS era recebida, normalizada e selecionada; a CSP continha `img-src 'self' asset: data:` e bloqueava a imagem remota. | Corrigido em código; validação manual pendente | `https:` foi adicionado somente a `img-src`; requer confirmação em WebView2 com GIF/WebP reais e origem que aceite `no-referrer`. |
| Prévia de arquivo/URL no Windows | A implementação usa assinatura local e `<img>` nativo; URL remota não é baixada por proxy. | Não testado | Requer verificação real de GIF/WebP animados, arquivo PNG, URL expirada e CORS. |

**Passos para importação P0.2:** execute `pnpm dev`, arraste para a zona HTML5 um GIF, WebP ou PNG local de até 25 MB e confirme a prévia, a origem local, o formato confirmado e a limpeza. Depois, arme a colagem e cole uma URL HTTPS `.gif` e uma `.webp`, incluindo uma variante com query string e fragmento; confirme que a origem aparece como remota e que o formato/animação são inferidos, sem que a URL integral seja exibida. Confirme a reprodução quando a origem permitir requisição sem referer. Repita com URL expirada ou bloqueada para confirmar o estado de erro. Teste separadamente navegador e Discord; registre apenas o mecanismo, formato, resultado e evidência sanitizada.

## Compartilhamento: GIFTray → destino

| Destino | Copiar URL | Copiar mídia/arquivo | Animação preservada? | Resultado |
|---|---|---|---|---|
| Discord Desktop | — | — | — | Não testado |
| Discord Web | — | — | — | Não testado |
| WhatsApp Web | — | — | — | Não testado |
| Telegram Desktop/Web | — | — | — | Não testado |
| Explorador de Arquivos (colar arquivo) | Não aplicável | — | — | Não testado |

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

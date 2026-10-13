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

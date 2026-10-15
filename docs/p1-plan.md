# P1 — Plano da biblioteca local persistente

> **Estado:** proposta para aprovação antes do primeiro incremento de código.
> **Base:** `develop` em `aad8e86`, após as pesquisas pós-P0. Este plano não implementa SQLite, armazenamento, extensões ou integração Shell.

## 1. Objetivo e escopo

A P1 transforma a captura explícita já experimentada na P0 em uma biblioteca local durável. O fluxo mínimo é:

```text
entrada explícita → validação no processo nativo → arquivo controlado + metadados
                → lista após reinício → cópia por ID para CF_HDROP
```

Ao fim da P1, o GIFTray deve poder receber GIF, WebP (animado ou estático) e PNG por uma ação explícita, validar o limite de 25 MB e a assinatura, guardar os bytes originais, reabrir os itens depois de reiniciar e copiar um item salvo como arquivo. A organização avançada vem somente depois desse percurso completo.

### Incluído

- biblioteca offline de arquivos locais controlados pelo aplicativo;
- metadados locais e listagem inicial;
- importação de arquivo local e URL HTTPS, esta última pelo downloader seguro da P0;
- cópia de item persistido por identificador estável;
- estados de falha, item ausente e recuperação no próximo início;
- base para busca local, favoritos e coleções posteriores.

### Explicitamente fora

- extensão Chrome/Edge, Native Messaging e menu do Explorer;
- automação, injeção ou integração não oficial com Discord;
- nuvem, conta, sincronização, backend remoto, plugins ou IA;
- descoberta de GIFs, edição ou conversão de mídia;
- prometer suporte universal de animação nos aplicativos de destino.

As integrações pesquisadas em [context-menu-research.md](context-menu-research.md) devem consumir a futura importação persistente; não devem definir seu contrato nem antecipá-la.

## 2. Princípios e invariantes

1. **A biblioteca é diferente do cache.** `clipboard-output` permanece um cache temporário de 24 horas/100 MB. Um item da biblioteca nunca depende dele, e a limpeza do cache não apaga mídia persistida.
2. **O Rust é a fronteira de confiança.** URL, caminho, `File`, HTML e futura mensagem externa são dados não confiáveis. O backend confirma tamanho, assinatura, destino e identidade antes de persistir.
3. **O item salvo é independente da origem.** A biblioteca guarda uma cópia dos bytes em diretório controlado; alterar, remover ou substituir o arquivo original não altera o item.
4. **Nenhum estado transitório é persistido.** Não salvar `object:` URL, `File` do WebView, `MediaPreviewState`, caminho do cache de clipboard ou estado visual.
5. **Falha de importação não cria item.** Arquivo parcial, assinatura inválida, erro de escrita ou erro de banco retornam falha sanitizada e são limpos ou recuperados na próxima abertura.
6. **Indisponibilidade posterior não é falha de importação.** Item que antes foi válido, mas cujo arquivo controlado desapareceu, mantém seus metadados e é marcado como indisponível.
7. **O frontend não escolhe caminho de saída.** Ele envia um ID validável ou uma origem explícita; somente o backend resolve caminhos sob o diretório da biblioteca.
8. **Sem logs sensíveis.** Não registrar bytes, caminhos privados ou URL original que possa conter token/assinatura.

## 3. Contratos de domínio propostos

Os nomes são contratos de responsabilidade, não exigência de criar um pacote `core`, store global ou repositório genérico.

| Conceito | Responsabilidade | Persistido? |
|---|---|---|
| `ReceivedInput` | Dados brutos de drop/colagem já recebidos pela WebView. | Não |
| `MediaPreviewState` | Seleção, prévia e `object:` URL da tela atual. | Não |
| `ImportRequest` | Pedido explícito para importar um arquivo local selecionado ou URL remota. Declara a origem, sem carregar detalhes de UI. | Não |
| `ImportProgress` | Estado efêmero `validating`, `copying`, `downloading`, `committing` ou `failed`; inicialmente sem percentual prometido. | Não |
| `ImportResult` | `Imported { item }`, `AlreadyPresent { item }` ou falha sanitizada. | Não |
| `LibraryItem` | Identidade, arquivo controlado e metadados para listar, pré-visualizar e copiar. | Sim |

### `LibraryItem` mínimo

```text
LibraryItem
  id: UUID textual estável
  format: gif | webp | png                 # confirmado por assinatura
  storage_key: nome opaco gerado pelo backend, não um caminho recebido da UI
  byte_size: inteiro positivo, até 25 MiB
  content_sha256: digest dos bytes originais
  source_kind: local_file | remote_url | external_request
  original_name: opcional e sanitizado para apresentação
  imported_at: instante UTC
  availability: available | missing
  optional metadata: reserved for title/tags/favorite posteriores
```

`source_kind` é uma classificação, não uma URL reproduzível. A P1 não persiste URL remota completa por padrão; em particular, URL assinada, query com token, fragmento ou credencial nunca entra no banco nem em log. Uma referência de origem reutilizável só poderá ser acrescentada depois por decisão explícita de produto e com política de privacidade própria.

### Estados e transições

```text
nenhum pedido
  → validating
  → copying | downloading
  → committing
  → imported | already-present | failed

LibraryItem available → missing              # arquivo controlado não é encontrado na verificação
LibraryItem missing   → available             # somente após recuperação comprovada, não por suposição
LibraryItem available | missing → deleted     # exclusão explícita e concluída
```

O estado `missing` preserva identificação e futura organização. Não há item para uma URL bloqueada, timeout, arquivo inválido ou cópia que falhou antes do commit.

## 4. Persistência: SQLite e diretório de mídia

### Decisão proposta

Usar SQLite no processo Rust para metadados locais e um diretório de arquivos separado, ambos sob o diretório de dados do aplicativo retornado por `AppHandle::path().app_data_dir()`:

```text
<app_data_dir>/
  library.sqlite3
  media/
    <uuid>.gif | <uuid>.webp | <uuid>.png
    .<uuid>.<ext>.part
```

`app_cache_dir()/clipboard-output` não é parte dessa árvore. `storage_key` corresponde somente ao nome opaco em `media/`; o caminho absoluto é reconstruído pelo backend, depois de validar o ID e o diretório controlado.

SQLite é adequado para as primeiras consultas locais (lista por data, busca posterior e metadados opcionais), migrações versionadas e restrições de unicidade. A implementação deve usar acesso local direto no Rust, sem plugin de banco exposto à WebView. A escolha de crate e flags de build fica para a implementação, mas a proposta é uma integração SQLite Rust pequena, mantida e sem serviço externo; ela precisa ser aprovada antes de adicionar dependência.

### Esquema inicial proposto

```sql
schema_migrations(version INTEGER PRIMARY KEY, applied_at INTEGER NOT NULL);

library_items(
  id TEXT PRIMARY KEY,
  storage_key TEXT NOT NULL UNIQUE,
  format TEXT NOT NULL CHECK (format IN ('gif', 'webp', 'png')),
  byte_size INTEGER NOT NULL CHECK (byte_size > 0 AND byte_size <= 26214400),
  content_sha256 TEXT NOT NULL UNIQUE,
  source_kind TEXT NOT NULL CHECK (source_kind IN ('local_file', 'remote_url', 'external_request')),
  original_name TEXT NULL,
  imported_at INTEGER NOT NULL,
  availability TEXT NOT NULL CHECK (availability IN ('available', 'missing'))
);
```

O primeiro esquema não cria colunas de favorito, tags, coleções ou URL original. Migrações são numeradas, aplicadas uma vez dentro de transação SQLite e testadas desde banco vazio e versão anterior suportada. Chaves estrangeiras devem ser habilitadas em cada conexão quando tabelas relacionadas surgirem.

### Consistência entre banco e filesystem

Uma transação SQLite não engloba `rename`, cópia, falta de espaço ou remoção no filesystem. Em vez de prometer atomicidade impossível, a P1 usa uma sequência recuperável no mesmo volume:

1. criar `.<uuid>.<ext>.part` dentro de `media/`;
2. copiar os bytes, contar o teto, calcular digest e confirmar assinatura antes de publicar;
3. se o digest já existir, remover `.part` e retornar `AlreadyPresent` com o item existente;
4. renomear o `.part` validado para o nome final opaco;
5. em transação SQLite, inserir o item com `availability = available`;
6. se a inserção falhar, tentar remover o arquivo final; se a remoção também falhar, ele será órfão detectável na próxima abertura.

Na inicialização, antes de oferecer a lista, a recuperação deve: remover `.part` abandonado; comparar `storage_key` do banco com a árvore `media/`; marcar registro sem arquivo como `missing`; remover arquivos finais órfãos não referenciados pelo banco. A última remoção ocorre somente dentro do diretório canônico controlado pelo GIFTray. Falha de varredura é conservadora: registrar erro sanitizado e não apagar candidatos incertos.

Operações de gravar, importar, recuperar e excluir precisam de exclusão mútua no processo. SQLite protege a base, mas não protege a sequência com filesystem; o módulo da biblioteca deve serializar essa transação lógica, assim como o cache de clipboard já serializa publicação e limpeza.

### Exclusão proposta

Excluir é uma ação explícita: localizar o item por ID, remover o arquivo controlado e remover o registro. Se o arquivo já não existir, remover o registro ainda é permitido depois de confirmar que pertence à biblioteca; se a remoção de arquivo falhar, não apagar o registro e informar a falha. A recuperação posterior trata órfãos, não transforma uma exclusão incompleta em sucesso.

## 5. Política de importação

### Arquivo local

O caminho de importação permanente deve preferir uma origem nativa com caminho de arquivo (seletor de arquivo e, depois de validado em Windows, drag-and-drop nativo). O backend abre o arquivo, limita a leitura a 25 MiB, identifica GIF/WebP/PNG por assinatura e copia os bytes sem conversão para `media/`.

Não confiar em extensão, MIME, nome ou caminho vindo do frontend. Um arquivo removido, trocado ou inacessível entre a seleção e a cópia falha sem item. A prévia HTML5/P0 pode continuar ajudando a escolher a mídia, mas não prova a disponibilidade de um caminho nativo nem deve ser persistida.

`File` recebido somente no WebView não deve ser convertido em caminho suposto nem transportado como `number[]` de até 25 MB. A adaptação exata para colagem/drag HTML5 ficará em P1.3, depois de uma prova Windows que preserve a fronteira nativa; enquanto isso, o seletor nativo fornece o fluxo completo verificável.

### URL remota

O comando de importação de URL chama o downloader P0 existente; não cria um segundo cliente HTTP. Permanecem obrigatórios HTTPS na porta 443, DNS público integralmente validado e vinculado à conexão, TLS/Rustls, ausência de proxy automático, redirects manuais, timeout global e teto efetivo de 25 MiB.

Somente bytes integralmente recebidos e com assinatura confirmada entram na mesma transação de `media/`. A URL pode expirar ou ser bloqueada depois; isso não afeta o item já salvo, pois a biblioteca guarda os bytes. Se o download falhar, não existe item parcial e a mensagem não expõe URL assinada.

### Duplicatas — proposta a aprovar

Deduplicar por `SHA-256` dos bytes após a validação, não por URL, nome ou extensão.

- mesmo conteúdo por arquivo e URL diferentes: conservar um único `LibraryItem` e retornar `AlreadyPresent` com seu ID;
- mesma URL que passou a servir bytes diferentes: importar novo item;
- mesma imagem recodificada: importar item distinto, porque os bytes e o formato podem ter diferenças relevantes;
- não atualizar nome, origem ou metadados de item existente silenciosamente.

Isso reduz espaço e evita duplicata óbvia sem assumir que uma URL identifica conteúdo. O custo é calcular e armazenar digest e definir a dependência criptográfica correspondente; o comportamento de UX de `AlreadyPresent` precisa de aprovação de produto antes de ser fixado.

## 6. Clipboard de itens persistidos

Adicionar um comando específico, conceitualmente `copy_library_item(id)`, em vez de reutilizar a API que recebe `fileName` e `bytes` da WebView.

1. o frontend fornece somente o ID do item selecionado;
2. o backend valida o ID, consulta o banco e resolve o caminho abaixo de `media/`;
3. confirma disponibilidade, teto e assinatura do arquivo antes de publicar;
4. cria uma cópia de saída no cache `clipboard-output` e reutiliza a publicação `CF_HDROP` existente;
5. retorna sucesso somente depois da publicação; a UI não recebe caminho local nem bytes do item.

Assim, excluir o item da biblioteca após copiar não quebra imediatamente a referência que já está no clipboard: este aponta para a cópia temporária, cuja proteção P0 contra limpeza prematura continua válida. Não prometer rollback do clipboard depois de `EmptyClipboard`, pois a API Win32 não o oferece; erros de publicação precisam manter a semântica já documentada e nunca anunciar arquivo inválido ou parcial.

Copiar link permanece independente, mas a P1 não deve oferecer URL original para item persistido se ela não tiver sido armazenada de modo seguro. Não transportar arquivo persistido pelo IPC como `number[]`.

## 7. Experiência de uso planejada

A interface continua em Fluent UI React v9 e o estado continua local às features e ao `App`, sem store global.

- **Importar:** ação explícita para selecionar arquivo e, quando P1.2 chegar, adicionar URL; impedir duas importações concorrentes da mesma tela quando isso não puder ser representado com segurança.
- **Progresso:** mostrar estágio (`validando`, `copiando`, `baixando`, `salvando`) e erro acionável, sem prometer percentual até existir medição real.
- **Lista:** apresentar itens persistidos por data de importação, formato, nome opcional e disponibilidade; estado vazio orienta à importação.
- **Prévia:** manter animação quando o formato e o WebView permitirem. A forma de servir arquivo controlado à WebView deve aceitar apenas recursos pertencentes à biblioteca, nunca caminho arbitrário da UI.
- **Teclado:** foco previsível do botão de importar para a lista, seleção visível, Enter para copiar e mensagens acessíveis de sucesso/falha.
- **Falhas:** texto sanitizado; não mostrar URL assinada, caminho privado ou detalhes internos de DNS/SQLite ao usuário.

P1 não redesenha a aplicação inteira nem substitui o atalho global ou o tema. A superfície de diagnóstico P0 será substituída gradualmente por uma experiência de importação, preservando os classificadores e testes que continuam úteis.

## 8. Módulos previstos e limites

| Área | Papel previsto | Limite deliberado |
|---|---|---|
| `src-tauri/src/library.rs` | Caso de uso de biblioteca: migração, importação, recuperação, listagem, exclusão e resolução interna por ID. | Não virar repositório genérico nem abstração de banco para toda a aplicação. Pode ser dividido apenas quando uma responsabilidade concreta surgir. |
| `src-tauri/src/clipboard.rs` | Ganhar caminho interno de cópia de item persistido para o cache/CF_HDROP. | Preservar cache, proteção de caminhos e testes P0; não reescrever Win32. |
| `src-tauri/src/remote_media.rs` | Continuar como único downloader remoto seguro. | Não refatorar segurança/rede para P1. |
| `src/platform/tauri/library.ts` | Adaptador TypeScript tipado para comandos de biblioteca. | Não expor caminho arbitrário ou bytes. |
| `src/features/media-library/` | UI e estado local para lista, importação e seleção. | Não criar store global; a prévia transitória segue separada. |
| `src/features/media-import/` | Normalização e prévia de entrada já existente. | Não persistir `MediaPreviewState` nem transformar objeto `File` em contrato de banco. |

## 9. Incrementos propostos

### P1.1 — fundação persistente, importação local e lista

| Aspecto | Plano |
|---|---|
| Escopo | Aprovar contrato; criar esquema SQLite/migração, diretório `media/`, recuperação inicial, importação por seletor nativo de arquivo, deduplicação proposta e listagem após reinício. |
| Módulos previstos | `library.rs`, registro de comandos em `lib.rs`, adaptador Tauri tipado e feature de biblioteca mínima; mudança pontual de composição em `App`. |
| Testes automatizados | Migração vazia/reexecução; assinatura/teto; bytes preservados; UUID/storage key; hash e duplicata; falhas de escrita/rename/banco; `.part`; órfão; arquivo ausente; listagem ordenada; ID e caminho malformados rejeitados. |
| Aceitação | GIF/WebP/PNG válido local aparece uma vez, persiste após reinício e é independente do arquivo de origem; inválido/maior que 25 MiB não cria registro nem arquivo final. |
| Manuais Windows | Selecionar arquivo com Unicode e disco diferente; reiniciar app; verificar GIF/WebP/PNG e estado vazio em Windows 10 22H2 e Windows 11. |
| Dependências/riscos | Aprovação de SQLite, UUID e digest; permissões Tauri para seletor/arquivos; diferença WebView2 e acesso nativo a arquivos. |
| Fora | URL remota, copiar item persistido, busca textual, favoritos, coleções, extensões/Shell. |

### P1.2 — URL segura, reabertura e cópia por ID

| Aspecto | Plano |
|---|---|
| Escopo | Importar URL pelo `remote_media` P0, reabrir itens `available`/`missing`, criar `copy_library_item(id)` e utilizar a cópia temporária já protegida do CF_HDROP. |
| Módulos previstos | Caso de uso em `library.rs`; adaptação mínima e interna em `clipboard.rs`; adaptador TypeScript e ação de cópia na feature de biblioteca. |
| Testes automatizados | Pedido remoto usa o downloader existente; falha/HTML/timeout não cria item; arquivo persistido mantém bytes; ID ausente/malformado e item `missing` não publicam; exclusão após cópia preserva o cache de clipboard. |
| Aceitação | Item salvo local/remoto reaparece após reinício e pode ser copiado sem envio de bytes/caminhos pela UI; falha não comunica sucesso. |
| Manuais Windows | Copiar GIF/WebP/PNG persistidos para Explorer e Discord; fechar GIFTray e colar; excluir item depois de copiar; repetir nos dois Windows. |
| Dependências/riscos | A limitação Win32 de não haver rollback após `EmptyClipboard`; compatibilidade de animação continua sendo do destino; sem URL persistida por padrão não há “copiar link” para esses itens. |
| Fora | Busca por texto, capture HTML5/paste permanente e organização. |

### P1.3 — experiência de importação e busca local mínima

| Aspecto | Plano |
|---|---|
| Escopo | Substituir a superfície diagnóstica pela importação explícita de produto; integrar seleção de URL, feedback por estágio, lista navegável por teclado e busca local sobre campos já existentes. Investigar adaptador seguro para drop/colagem WebView sem payload grande por IPC. |
| Módulos previstos | Evolução de `media-library`, reaproveitamento seletivo de `media-import` e `input-diagnostics`, adaptador Tauri de biblioteca. |
| Testes automatizados | Estados de importação, foco/teclado, query vazia/normalizada, erro sanitizado e ausência de URL/caminho no estado persistido. |
| Aceitação | Usuário importa sem tela diagnóstica, entende falhas e encontra um item salvo por seu nome opcional/formato; operações concorrentes não deixam seleção inválida. |
| Manuais Windows | Drop/colagem e seletor real em Win10/Win11; DPI, foco, leitor de tela básico, URL expirada/bloqueada e mídia animada na prévia. |
| Dependências/riscos | A WebView pode não fornecer caminho confiável para todo `File`; não assumir equivalência com drag nativo até a matriz manual provar. |
| Fora | Favoritos, tags e coleções; extensão de navegador; Explorer. |

### P1.4 — organização progressiva

| Aspecto | Plano |
|---|---|
| Escopo | Favorito, nome/título opcional, tags e coleções locais somente após estabilizar item e importação. |
| Módulos previstos | Migração SQLite específica, consultas de busca e componentes de organização. |
| Testes automatizados | Migração, integridade de relações, busca/ordenação, exclusão e preservação de item indisponível. |
| Aceitação | Organizar é opcional; captura/importação rápida continua possível sem metadados obrigatórios. |
| Manuais Windows | Navegação por teclado, foco, tema, itens indisponíveis e biblioteca grande representativa. |
| Dependências/riscos | Decisões de UX para coleções e exclusão; crescimento de esquema sem cloud. |
| Fora | Sincronização, compartilhamento social e descoberta de mídia. |

## 10. Matriz de testes da P1

| Área | Automatizado determinístico | Manual Windows ainda necessário |
|---|---|---|
| Assinatura e limite | GIF/WebP/PNG válidos; HTML/assinatura inválida; vazio; 25 MiB e acima. | Arquivos reais de cada formato, incluindo WebP animado. |
| Armazenamento | Bytes idênticos, `.part`, rename, falha de escrita, falta de espaço simulada, órfãos e recuperação. | Disco com pouco espaço, permissões e antivírus em Win10/Win11. |
| SQLite | Migrações, constraints, duplicata, reabertura, `missing`, concorrência interna e exclusão incompleta. | Atualização de versão instalada sobre biblioteca existente. |
| Remoto | Reutilização de testes determinísticos P0 para URL/DNS/redirect/timeout e ausência de item em erro. | TLS, DNS, servidor real permitido, URL expirada e origem autenticada sem registrar segredo. |
| Clipboard | ID inválido/missing não publica; arquivo validado gera cache; retenção do cache após exclusão do item. | CF_HDROP real, fechar app, Explorer/Discord e limites já registrados na matriz P0. |
| UI | Estados, seleção, teclas, feedback e sanitização de mensagens. | Foco, tema, DPI, WebView2, alto contraste e animação por formato. |

Não declarar testes simulados como prova de CF_HDROP, Explorer, Discord, Windows 10 ou Windows 11. A [matriz P0](p0-test-matrix.md) continua sendo a evidência histórica do downloader e clipboard; a P1 deve acrescentar sua própria matriz de execução quando o código existir.

## 11. Decisões que exigem aprovação

| Decisão | Proposta | Motivo para aprovar antes do código |
|---|---|---|
| Banco local | SQLite no Rust e arquivo em `app_data_dir`, sem plugin de banco para WebView. | Introduz dependência, migração e dados persistentes do usuário. |
| Identidade | UUID opaco por item + `SHA-256` como identidade de conteúdo. | Define formato durável e dependência de geração/digest. |
| Duplicatas | Uma cópia por digest, retorno `AlreadyPresent`, sem mutar metadados existentes. | Muda a expectativa de quem tenta salvar o mesmo arquivo duas vezes. |
| URL de origem | Não persistir URL completa por padrão. | Privacidade versus conveniência de “copiar link” posteriormente. |
| Exclusão | Excluir bytes e registro, mantendo CF_HDROP por cópia temporária protegida. | Define expectativa de disponibilidade depois de apagar um item. |
| Entrada inicial | Seletor de arquivo nativo para P1.1; drop/colagem HTML5 permanente só após prova de caminho/stream seguro. | Evita enviar 25 MiB como arrays pelo IPC ou assumir capacidade do WebView. |
| Prévia de arquivo persistido | Servir somente recursos mapeados por ID/controlados pelo backend; escolher mecanismo Tauri estreito em P1.2. | Impede transformar a UI em leitor de caminho arbitrário. |

## 12. Riscos remanescentes

- Falha de energia entre `rename` e commit SQLite pode deixar órfão; a recuperação é compensatória, não transação distribuída.
- O diretório de dados pode ser apagado, bloqueado por outro processo ou afetado por disco cheio; o banco deve refletir `missing` e não inventar bytes.
- A proteção do cache CF_HDROP continua conservadora e não substitui smoke test nativo de falhas Win32.
- URLs de origem podem expirar, exigir autenticação ou bloquear download; o item só existe após bytes válidos já terem sido guardados.
- A P0 ainda não fornece cobertura específica registrada para Windows 10 22H2 e Windows 11; cada incremento preserva essa validação manual.
- SQLite torna as futuras migrações responsabilidade do produto; o esquema inicial deve ficar deliberadamente pequeno.

## 13. Próximo passo recomendado

Antes de iniciar P1.1, aprovar as sete decisões da seção 11 e registrar as versões/diretrizes das dependências escolhidas. Em seguida, implementar apenas P1.1 em uma feature branch, com testes de filesystem/SQLite determinísticos e smoke tests Windows separados. Não iniciar extensão, Explorer ou P1.4 como condição para validar a biblioteca mínima.

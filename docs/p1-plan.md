# P1 — Plano da biblioteca local persistente

> **Estado:** direções de produto e arquitetura aprovadas antes do primeiro incremento de código. A escolha justificada de crates e versões fica para a implementação.
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
7. **O frontend não escolhe caminhos.** Para arquivo local, o seletor nativo autoriza a escolha diretamente ao backend; a UI não envia um caminho arbitrário por IPC. Para cópia e prévia, ela fornece somente um ID validável e o backend resolve recursos sob o diretório da biblioteca.
8. **Sem logs sensíveis.** Não registrar bytes, caminhos privados ou URL original que possa conter token/assinatura.

## 3. Contratos de domínio aprovados

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

### Decisões aprovadas

Usar SQLite exclusivamente no processo Rust para metadados locais e um diretório de arquivos separado, ambos sob o diretório local de dados retornado por `AppHandle::path().app_local_data_dir()` no Windows:

```text
<app_local_data_dir>/
  library/
    library.sqlite3
    media/
      <uuid>.gif | <uuid>.webp | <uuid>.png
      .<uuid>.<ext>.part
```

Esses são dados permanentes do usuário, não cache descartável. `app_cache_dir()/clipboard-output` não é parte dessa árvore. `storage_key` corresponde somente ao nome opaco em `media/`; o caminho absoluto é reconstruído pelo backend, depois de validar o ID e o diretório controlado.

SQLite é adequado para as primeiras consultas locais (lista por data, busca posterior e metadados opcionais), migrações versionadas e restrições de unicidade. A implementação deve usar acesso local direto no Rust, sem plugin de banco exposto à WebView. A escolha de crate e flags de build fica para P1.1, com justificativa de manutenção, distribuição Windows e testes; não exige nova aprovação da direção arquitetural.

### Esquema inicial planejado

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
6. se a inserção falhar, tentar remover o arquivo final; se a remoção também falhar, preservá-lo como arquivo final sem registro para inspeção não destrutiva na próxima abertura.

Na inicialização, a recuperação ocorre somente após obter o bloqueio exclusivo da biblioteca e antes de oferecer a lista. Ela deve:

1. abrir SQLite; se isso falhar, interromper a recuperação sem limpar arquivo algum;
2. remover `.part` abandonado cujo nome corresponda ao padrão interno, pois ele nunca é arquivo publicado;
3. comparar registros com a árvore `media/` e marcar registro sem arquivo como `missing` em transação SQLite;
4. preservar arquivo final sem registro: ele é desconhecido e não pode ser apagado automaticamente nesta fase.

Arquivos finais não referenciados podem ser resultado de queda entre `rename` e o commit, mas também podem exigir investigação. A P1 não os apaga por idade, nome ou mera ausência no banco; uma reparação destrutiva futura exigirá um critério explícito e evidência adicional. Toda inspeção de caminho continua limitada ao diretório canônico `library/media`.

SQLite protege a base, mas não protege a sequência com filesystem. O módulo da biblioteca deve combinar mutex no processo com bloqueio exclusivo no arquivo dedicado `library/library.lock` para recuperação, importação e exclusão. Enquanto uma instância mantém a operação, outra instância não pode iniciar recuperação nem mutação; ela recebe estado de biblioteca em uso e não faz varredura ou limpeza. O bloqueio é liberado pelo sistema se o processo encerrar, permitindo que a próxima abertura recupere `.part` abandonado. Essa solução pequena cobre a concorrência entre instâncias sem criar framework genérico.

### Exclusão aprovada

Excluir é uma ação explícita: localizar o item por ID, remover o arquivo controlado e remover o registro. Se o arquivo já não existir, remover o registro ainda é permitido depois de confirmar que pertence à biblioteca; se a remoção de arquivo falhar, não apagar o registro e informar a falha. A recuperação posterior preserva arquivo final desconhecido e não transforma uma exclusão incompleta em sucesso.

## 5. Política de importação

### Arquivo local

O primeiro caminho de importação permanente é um seletor nativo que autoriza a escolha diretamente ao backend Rust. A UI solicita a ação de selecionar/importar, mas não recebe uma API IPC genérica como `import_path(path)`: o comando nativo abre o seletor, obtém a escolha autorizada e chama internamente a importação por caminho. A função interna por caminho pode existir para testes e futuras integrações nativas autorizadas, porém não é exposta à WebView.

O backend abre o arquivo autorizado, limita a leitura a 25 MiB, identifica GIF/WebP/PNG por assinatura e copia os bytes sem conversão para `media/`.

Não confiar em extensão, MIME, nome ou caminho vindo do frontend. Um arquivo removido, trocado ou inacessível entre a seleção e a cópia falha sem item. A prévia HTML5/P0 pode continuar ajudando a escolher a mídia, mas não prova a disponibilidade de um caminho nativo nem deve ser persistida.

`File` recebido somente no WebView não deve ser convertido em caminho suposto nem transportado como `number[]` de até 25 MB. A adaptação exata para colagem/drag HTML5 ficará em P1.3, depois de uma prova Windows que preserve a fronteira nativa; enquanto isso, o seletor nativo fornece o fluxo completo verificável.

### URL remota

O comando de importação de URL chama o downloader P0 existente; não cria um segundo cliente HTTP. Permanecem obrigatórios HTTPS na porta 443, DNS público integralmente validado e vinculado à conexão, TLS/Rustls, ausência de proxy automático, redirects manuais, timeout global e teto efetivo de 25 MiB.

Somente bytes integralmente recebidos e com assinatura confirmada entram na mesma transação de `media/`. A URL pode expirar ou ser bloqueada depois; isso não afeta o item já salvo, pois a biblioteca guarda os bytes. Se o download falhar, não existe item parcial e a mensagem não expõe URL assinada.

### Duplicatas — decisão aprovada

Deduplicar por `SHA-256` dos bytes após a validação, não por URL, nome ou extensão.

- mesmo conteúdo por arquivo e URL diferentes: conservar um único `LibraryItem` e retornar `AlreadyPresent` com seu ID;
- mesma URL que passou a servir bytes diferentes: importar novo item;
- mesma imagem recodificada: importar item distinto, porque os bytes e o formato podem ter diferenças relevantes;
- não atualizar nome, origem ou metadados de item existente silenciosamente.

Isso reduz espaço e evita duplicata óbvia sem assumir que uma URL identifica conteúdo. O custo é calcular e armazenar digest e definir a dependência criptográfica correspondente; a escolha exata fica para P1.1, com justificativa no commit que a introduzir.

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
| `src-tauri/src/library.rs` | Caso de uso de biblioteca: migração, seleção nativa autorizada, importação interna, recuperação, listagem, exclusão e resolução interna por ID. | Não virar repositório genérico nem abstração de banco para toda a aplicação. Pode ser dividido apenas quando uma responsabilidade concreta surgir. |
| `src-tauri/src/clipboard.rs` | Ganhar caminho interno de cópia de item persistido para o cache/CF_HDROP. | Preservar cache, proteção de caminhos e testes P0; não reescrever Win32. |
| `src-tauri/src/remote_media.rs` | Continuar como único downloader remoto seguro. | Não refatorar segurança/rede para P1. |
| `src/platform/tauri/library.ts` | Adaptador TypeScript tipado para comandos de biblioteca. | Não expor caminho arbitrário ou bytes. |
| `src/features/media-library/` | UI e estado local para lista, importação e seleção. | Não criar store global; a prévia transitória segue separada. |
| `src/features/media-import/` | Normalização e prévia de entrada já existente. | Não persistir `MediaPreviewState` nem transformar objeto `File` em contrato de banco. |

## 9. Incrementos da P1

### P1.1 — fundação persistente, importação local e lista

| Aspecto | Plano |
|---|---|
| Escopo | Aplicar o contrato aprovado: criar esquema SQLite/migração, diretório `media/`, recuperação inicial, importação por seletor nativo de arquivo, deduplicação por conteúdo e listagem após reinício. |
| Módulos previstos | `library.rs`, registro de comandos em `lib.rs`, adaptador Tauri tipado e feature de biblioteca mínima; mudança pontual de composição em `App`. |
| Testes automatizados | Migração vazia/reexecução; assinatura/teto; bytes preservados; UUID/storage key; hash e duplicata; falhas de escrita/rename/banco; falha ao abrir SQLite não limpa arquivos; limpeza de `.part`; arquivo final sem registro preservado; registro sem arquivo vira `missing`; listagem ordenada; seleção não expõe caminho arbitrário; bloqueio entre instâncias. |
| Aceitação | Importa arquivo local GIF/WebP/PNG válido pelo seletor nativo, salva bytes e metadados, lista e recupera após reinício, e é independente do arquivo de origem. Inválido/maior que 25 MiB não cria registro nem arquivo final. P1.1 não exige prévia animada da biblioteca. |
| Manuais Windows | Selecionar arquivo com Unicode e disco diferente; reiniciar app; verificar GIF/WebP/PNG e estado vazio em Windows 10 22H2 e Windows 11. |
| Dependências/riscos | Escolha justificada de crates SQLite, UUID e digest; permissões Tauri para seletor/arquivos; diferença WebView2 e acesso nativo a arquivos. |
| Fora | URL remota, copiar item persistido, busca textual, favoritos, coleções, extensões/Shell. |

### P1.2 — URL segura, reabertura e cópia por ID

| Aspecto | Plano |
|---|---|
| Escopo | Importar URL pelo `remote_media` P0, visualizar arquivo persistido por recurso restrito da biblioteca, reabrir itens `available`/`missing`, criar `copy_library_item(id)` e utilizar a cópia temporária já protegida do CF_HDROP. |
| Módulos previstos | Caso de uso em `library.rs`; adaptação mínima e interna em `clipboard.rs`; adaptador TypeScript e ação de cópia na feature de biblioteca. |
| Testes automatizados | Pedido remoto usa o downloader existente; falha/HTML/timeout não cria item; arquivo persistido mantém bytes; ID ausente/malformado e item `missing` não publicam; exclusão após cópia preserva o cache de clipboard. |
| Aceitação | Importa URL remota sob a política P0; item salvo local/remoto reaparece após reinício, pode ser visualizado como arquivo persistido e copiado por ID sem envio de bytes/caminhos pela UI. A animação é preservada quando o formato, WebView e aplicativo de destino a suportarem; falha não comunica sucesso. |
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
| Armazenamento | Bytes idênticos, `.part`, rename, falha de escrita, falta de espaço simulada, arquivo final sem registro preservado e recuperação não destrutiva. | Disco com pouco espaço, permissões e antivírus em Win10/Win11. |
| SQLite | Migrações, constraints, duplicata, reabertura, `missing`, bloqueio entre instâncias e exclusão incompleta. | Atualização de versão instalada sobre biblioteca existente. |
| Remoto | Reutilização de testes determinísticos P0 para URL/DNS/redirect/timeout e ausência de item em erro. | TLS, DNS, servidor real permitido, URL expirada e origem autenticada sem registrar segredo. |
| Clipboard | ID inválido/missing não publica; arquivo validado gera cache; retenção do cache após exclusão do item. | CF_HDROP real, fechar app, Explorer/Discord e limites já registrados na matriz P0. |
| UI | Estados, seleção, teclas, feedback e sanitização de mensagens. | Foco, tema, DPI, WebView2, alto contraste e animação por formato. |

Não declarar testes simulados como prova de CF_HDROP, Explorer, Discord, Windows 10 ou Windows 11. A [matriz P0](p0-test-matrix.md) continua sendo a evidência histórica do downloader e clipboard; a P1 deve acrescentar sua própria matriz de execução quando o código existir.

## 11. Decisões aprovadas para P1

| Decisão | Direção aprovada | Limite de implementação |
|---|---|---|
| Banco local | SQLite exclusivamente no backend Rust, sob `app_local_data_dir()/library`, sem plugin de banco para WebView. | Escolher crate e versão em P1.1 com justificativa de manutenção, distribuição Windows e testes. |
| Identidade | UUID estável e opaco por item. | O gerador concreto deve ser justificado na implementação. |
| Conteúdo e duplicatas | SHA-256 dos bytes, uma cópia por digest e `AlreadyPresent`. | A implementação não trata URL, nome ou extensão como identidade. |
| Metadados de duplicata | `AlreadyPresent` não modifica metadados do item existente. | Nome, origem e organização não são atualizados silenciosamente. |
| URL de origem | URL completa não é persistida por padrão. | Não introduzir armazenamento/log de URL assinada sem nova decisão de privacidade. |
| Exclusão e clipboard | Exclusão lógica segura; CF_HDROP aponta para cópia temporária protegida, não para o arquivo da biblioteca. | Não prometer rollback após `EmptyClipboard`. |
| Entrada inicial | Seleção nativa autorizada para P1.1; drag-and-drop e colagem ficam planejados para P1.3. | Não expor importação de caminho arbitrário à WebView. |
| Prévia persistida | Prévia restrita a arquivos mapeados por ID e controlados pela biblioteca. | O mecanismo Tauri concreto é escolhido em P1.2 sem aceitar caminho arbitrário. |

## 12. Riscos remanescentes

- Falha de energia entre `rename` e commit SQLite pode deixar arquivo final sem registro; ele será preservado de forma conservadora até existir reparação explicitamente projetada, com possível custo de espaço.
- O diretório de dados pode ser apagado, bloqueado por outro processo ou afetado por disco cheio; o banco deve refletir `missing` e não inventar bytes.
- Várias instâncias não podem executar recuperação ou mutação simultaneamente; a segunda deve receber biblioteca em uso e não fazer limpeza. O comportamento de leitura simultânea será mantido restrito até ser justificado por testes.
- A proteção do cache CF_HDROP continua conservadora e não substitui smoke test nativo de falhas Win32.
- URLs de origem podem expirar, exigir autenticação ou bloquear download; o item só existe após bytes válidos já terem sido guardados.
- A P0 ainda não fornece cobertura específica registrada para Windows 10 22H2 e Windows 11; cada incremento preserva essa validação manual.
- SQLite torna as futuras migrações responsabilidade do produto; o esquema inicial deve ficar deliberadamente pequeno.

## 13. Próximo passo recomendado

As direções da seção 11 já estão aprovadas. Antes de iniciar P1.1, selecionar e justificar as crates e versões necessárias no escopo do incremento. Em seguida, implementar apenas P1.1 em uma feature branch, com testes de filesystem/SQLite determinísticos e smoke tests Windows separados. Não iniciar extensão, Explorer ou P1.4 como condição para validar a biblioteca mínima.

## 14. Registro de implementação P1.1.1

O primeiro incremento usa `rusqlite` com a feature `bundled`: a biblioteca SQLite acompanha o binário Rust, evitando depender de uma DLL SQLite instalada no Windows e sem expor qualquer plugin ou acesso ao banco para a WebView. `uuid` gera IDs v4 opacos, `sha2` calcula SHA-256 durante a cópia em streaming e `fs4` usa o lock exclusivo real do sistema operacional em `library.lock`; o arquivo do lock sozinho nunca é usado como indicador de posse.

`library.rs` é uma fronteira nativa interna, sem comando Tauri de importação. A inicialização cria e migra `library.sqlite3`, limpa somente `.part` com o padrão interno, marca registros cujo arquivo controlado sumiu como `missing` e preserva arquivos finais sem registro. A importação copia para `.part`, limita 25 MiB, confirma a assinatura e publica por `rename` antes do `INSERT`; uma falha de banco tenta remover o arquivo novo, mas preserva-o se essa remoção falhar.

Os testes deste incremento usam diretórios temporários isolados para esquema vazio e migração repetida, formatos aceitos, bytes/digest/UUID, duplicata, reabertura, ordenação, entradas inválidas, limite, falhas simuladas e recuperação. A validação manual em Windows 10/11 ainda é necessária antes de alegar comportamento em disco sob antivírus, falta de espaço ou encerramento real do processo.

## 15. Estabilização de exclusividade entre instâncias

O handle exclusivo de `library.lock` pertence à `Library` durante toda a vida da instância, compartilhado entre clones por `Arc`. O mutex associado à `Library` continua serializando operações locais, mas não reabre o lock do sistema operacional. Assim, uma segunda instância falha com `InUse` antes de abrir SQLite ou executar recovery; após o encerramento normal ou uma queda do primeiro processo, o sistema operacional libera o handle e uma nova inicialização pode recuperar a biblioteca. O teste de processos separados verifica a recusa, a preservação de `.part` e do SQLite e a reabertura posterior em diretório temporário.

## 16. Cópia de item persistido por ID

`copy_library_item` recebe somente o UUID opaco pelo IPC. O backend valida o ID, o estado `available`, a storage key interna, tamanho, assinatura e SHA-256 do arquivo antes de criar uma cópia limitada no cache `clipboard-output`. A publicação CF_HDROP continua apontando apenas para essa cópia temporária, reutilizando o limite, a retenção e a proteção do arquivo ativo já existentes. Caminhos, hashes, storage keys e bytes não retornam à WebView. Falhas de leitura, cache ou publicação não modificam o item permanente; testes automatizados cobrem a resolução, os formatos aceitos, a cópia em fluxo e a limpeza de temporários.

## 17. Biblioteca visual e prévia restrita

A WebView identifica mídia persistida somente pelo UUID público. O protocolo Tauri privado `library-media` aceita exclusivamente as rotas `thumbnail/<uuid>` e `preview/<uuid>`; cada pedido resolve o item pela biblioteca nativa e repete a validação de disponibilidade, storage key controlada, tamanho, assinatura e digest antes de responder. Não há comando que aceite caminho, nem listagem do diretório, e a CSP libera somente a origem local desse protocolo para imagens.

Miniaturas são PNGs estáticos gerados no backend a partir do primeiro quadro, com dimensão máxima de 240×160 e teto de alocação de 16 MiB. A prévia selecionada recebe os bytes originais e, portanto, permite que GIF e WebP animados sejam reproduzidos pelo WebView2 quando ele os suportar. Em movimento reduzido, a interface solicita somente a miniatura estática. Item `missing`, mídia alterada, falha de decodificação ou erro de carregamento apresentam fallback sem impedir a lista ou a ação já existente de copiar por UUID.

Os testes nativos cobrem rotas inválidas, traversal, item ausente, resposta e MIME controlados, falha de decodificação e geração de miniatura. Os testes de apresentação cobrem disponibilidade e movimento reduzido. Ainda são necessários smoke tests no Windows para animação GIF/WebP no WebView2, `prefers-reduced-motion`, DPI, temas claro/escuro/alto contraste e navegação com leitor de tela.

## 18. Importação HTTPS persistente

`import_remote_media(url)` recebe uma URL somente para entregá-la ao downloader P0 `remote_media::download`. A espera de DNS, TLS, redirects e corpo ocorre antes de qualquer lock de operação da biblioteca. Depois de receber todos os bytes sob as proteções já existentes (HTTPS/443, DNS público vinculado, sem proxy automático, redirects manuais, timeout de 30 segundos e 25 MiB), o comando chama a rotina nativa de persistência por `Read`.

Essa rotina é a mesma usada pelo seletor local: valida assinatura e tamanho, calcula SHA-256, grava `.part`, deduplica, publica por `rename` e registra SQLite sob o mutex e lock de instância. Uma mídia remota nova recebe `source_kind = remote_url` e `original_name = None`; a URL, inclusive query, token e credenciais, não entra no banco, arquivo, log ou retorno IPC. Duplicata retorna o item existente sem alterar seus metadados.

A interface envia somente a URL ao comando Tauri, bloqueia a importação por seletor e por URL enquanto uma já está em curso e preserva o texto em caso de erro. Os smoke tests pendentes em Windows incluem DNS/TLS reais permitidos, URL assinada expirada, feedback de erro sanitizado, reinício após importação e validação de foco/temas/DPI.

## 19. Busca local e nome personalizado

A migração SQLite V2 acrescenta `display_name` opcional a `library_items`, sem alterar `original_name`, UUID, hash, `source_kind`, storage key ou arquivo físico. A migração é transacional, repetível e mantém registros V1; `rename_library_item(id, display_name)` aceita somente o UUID e o texto, normaliza espaços, limita o nome a 120 caracteres Unicode e rejeita controles. Texto vazio remove o nome personalizado e restaura o fallback de nome original ou formato.

A busca é local e em memória, sobre a lista já carregada. Ela normaliza maiúsculas/minúsculas e diacríticos para comparar nome exibido, nome original e formato, sem criar FTS, índice ou endpoint de pesquisa. Renomear atualiza a lista local após o commit, portanto o filtro usa o novo nome imediatamente; seleção, prévia por UUID e cópia por UUID não mudam. Smoke tests pendentes: na edição inline, confirmar Enter para salvar, Escape em campo/salvar/cancelar sem fechar a prévia, e foco no novo botão Renomear ou na busca quando o filtro oculta o item; além de busca em bibliotecas grandes, DPI, temas e alto contraste em Windows 10 22H2 e Windows 11.

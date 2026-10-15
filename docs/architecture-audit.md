# Auditoria arquitetural pós-P0

> **Data da base auditada:** `develop` em `eebf9587a04334cf7acc0a5d44f370709bcd209e`.
> **Escopo:** leitura do código e das configurações atuais; nenhuma alteração de produção foi feita.
> **Método:** 41 arquivos-fonte foram examinados (35 em React/TypeScript e 6 em Rust), incluindo 8 arquivos de testes TypeScript e 23 testes Rust embutidos nos módulos nativos. Também foram revisados os seis documentos em `docs/`, as configurações Tauri/Cargo/pnpm e o workflow de CI.

## 1. Resumo executivo

A arquitetura atual é adequada para encerrar a P0 e iniciar uma P1 incremental. O código separa UI por feature, regras puras de inspeção de mídia, adaptadores Tauri e integrações nativas. O downloader e o cache de clipboard são deliberadamente mais detalhados porque concentram controles de segurança e integridade; o tamanho desses arquivos, por si só, não justifica uma divisão.

O principal trabalho de preparação para a P1 é definir uma fronteira explícita entre a entrada transitória da P0 (`File`, URL e object URL) e um item persistente da biblioteca. Hoje `App.tsx` encadeia captura diagnóstica, prévia temporária e saída de clipboard. Isso é correto para a prova técnica, mas não fornece ainda um contrato para identidade, origem, caminho local durável, estado de disponibilidade e metadados que a biblioteca precisará armazenar.

Não foi encontrado bloqueador de segurança ou integridade que exija reabrir P0. As limitações de interoperabilidade nativa continuam corretamente documentadas como testes manuais, e não como garantias automáticas.

## 2. Mapa atual de módulos

```text
React / TypeScript
app/App.tsx
  ├─ preferência de tema e atalho; composição das features
  ├─ input-diagnostics → shared/received-input.ts
  ├─ media-import (normalização, inspeção e prévia em memória)
  └─ clipboard-output → platform/tauri/clipboard.ts

platform/tauri/
  ├─ window.ts: comandos de janela e evento de foco
  ├─ input.ts: listener nativo de drag-and-drop
  └─ clipboard.ts: contratos IPC e clipboard textual

Rust / Tauri
lib.rs
  ├─ registro de plugins, comandos e limpeza inicial
  ├─ shortcuts.rs: estado do atalho global
  ├─ window.rs: janela principal e evento de abertura
  ├─ remote_media.rs: política URL/DNS, reqwest, redirects e streaming
  └─ clipboard.rs: validação final, cache temporário e CF_HDROP
```

Os dados seguem dois caminhos P0 distintos:

1. Arquivo/URL recebido → inspeção no frontend → prévia transitória → copiar link ou arquivo.
2. URL remota → comando Rust → validação DNS/HTTP → bytes limitados → validação de assinatura → cache temporário → `CF_HDROP`.

Não há banco, modelo de biblioteca ou persistência de itens P1. A busca em `features/gif-search/index.tsx` usa somente três exemplos estáticos, como previsto para a P0.

## 3. Pontos fortes confirmados

- **Limites de responsabilidade claros no frontend.** `app/App.tsx` compõe as features; chamadas Tauri ficam em `platform/tauri/`; regras sem DOM em `media-import/lib`, `input-diagnostics/diagnostics.ts` e `clipboard-output/lib`. A maioria dos testes TypeScript cobre precisamente esses módulos puros.
- **Entrada não confiável é tratada de forma defensiva.** `shared/received-input.ts` apenas lê dados recebidos explicitamente; `media.ts` aceita URLs HTTPS, não renderiza HTML e confirma assinaturas locais antes da cópia.
- **Fronteira nativa pequena e tipada.** Não há `invoke` espalhado pelas features: `platform/tauri/window.ts`, `input.ts` e `clipboard.ts` são adaptadores específicos.
- **Downloader remoto testável sem abrir proteção de produção.** `remote_media.rs` mantém a política URL/DNS e o transporte reqwest no caminho real, ao mesmo tempo que usa `MediaTransport`/`MediaBody` para exercitar redirects, timeout, erros e limite efetivo em testes determinísticos.
- **Segurança de rede preservada.** `remote_media.rs` exige HTTPS/443, rejeita host literal/credenciais e respostas DNS mistas ou não públicas, desabilita proxy e redirects automáticos e vincula endereços validados ao cliente reqwest.
- **Cache/clipboard transacional dentro dos limites Win32.** `clipboard.rs` reserva espaço antes de gravar, usa `.part` seguido de rename, só publica arquivo completo e remove o novo arquivo se a publicação falhar. O lock único e a consulta conservadora ao clipboard evitam limpeza insegura.
- **Configuração e CI proporcionais.** As capabilities não concedem permissões desnecessárias, e `.github/workflows/ci.yml` verifica frontend e Rust no Windows, sem secrets, release, deploy ou artefatos.

## 4. Problemas confirmados

### A-1 — Não existe contrato de item persistente entre a captura P0 e a futura biblioteca

- **Prioridade:** A — corrigir antes de iniciar P1 (decisão e corte inicial da implementação).
- **Severidade:** alta para evolução; não é falha funcional da P0.
- **Localização:** `apps/desktop/src/app/App.tsx` (aprox. linhas 43–99), `features/media-import/hooks/useMediaImport.ts` (20–115), `features/clipboard-output/*`.
- **Responsabilidade atual:** `MediaPreviewState` representa simultaneamente processamento, carregamento visual e a mídia que pode ser copiada; `App` passa essa estrutura diretamente de captura para saída.
- **Evidência:** `MediaPreviewState` contém `File` e `src` de object URL, ambos de vida curta. `MediaPreview` afirma que a mídia é mantida apenas em memória. `ClipboardOutput` toma decisões a partir desse mesmo estado transitório.
- **Impacto:** uma biblioteca persistente não pode usar `File` ou object URL como identidade; precisa separar fonte (arquivo local ou URL), representação persistida, disponibilidade e prévia. Sem essa decisão, P1 tende a vazar detalhes da prévia para banco, busca e coleções.
- **Solução mínima:** antes do primeiro fluxo P1, documentar e criar um contrato de domínio local mínimo para item salvo, sem escolher SQLite ainda: identificador, tipo de origem, localizador persistente, formato confirmado, metadados opcionais e estado de disponibilidade. A operação de salvar deve consumir o resultado validado da importação e devolver esse item; a prévia deve continuar sendo uma projeção transitória.
- **Risco da alteração:** médio. É uma mudança de fronteira de dados, mas pode começar sem migrar clipboard, downloader ou UI de busca.
- **Testes após a mudança:** conversão de arquivo/URL para item persistível; rejeição de objeto URL como dado salvo; reabertura de item local; estados de origem indisponível; regressão de copiar link/arquivo.

### A-2 — Captura de entrada é uma feature de diagnóstico, não um caso de uso de importação P1

- **Prioridade:** A — corrigir junto à primeira tela de importação P1.
- **Severidade:** média.
- **Localização:** `features/input-diagnostics/hooks/useInputDiagnostics.ts` (55–165), `InputDiagnostics.tsx` (13–48) e `App.tsx` (89–94).
- **Responsabilidade atual:** o hook mede eventos HTML5/Tauri, mostra diagnóstico e encaminha `ReceivedInput` opcionalmente à prévia.
- **Evidência:** o caminho nativo de drag-and-drop registra somente caminhos para diagnóstico e não chama `onInput`; o caminho HTML/paste chama `readReceivedInput`. A própria interface é denominada “Diagnóstico de importação”.
- **Impacto:** P1 precisará de uma captura orientada a salvar itens, mensagens ao usuário e políticas de origem; reaproveitar a tela de diagnóstico como tela de importação misturaria observabilidade experimental com fluxo de produto.
- **Solução mínima:** manter `ReceivedInput` e os classificadores puros, mas extrair na P1 uma interface de captura de importação que entregue um resultado normalizado. Deixar o diagnóstico como ferramenta P0 ou removê-lo somente quando uma experiência de importação equivalente existir.
- **Risco da alteração:** médio, pois drag-and-drop e paste têm particularidades de WebView/Tauri.
- **Testes após a mudança:** mesmos cenários de `DataTransfer`, colagem explicitamente armada, não captura de campos comuns e smoke test Windows para os dois perfis de drag-and-drop.

### B-1 — Transporte de bytes local pelo IPC é caro no limite permitido

- **Prioridade:** B — corrigir durante P1 se a cópia de arquivos continuar aceitando arquivos do WebView.
- **Severidade:** média.
- **Localização:** `platform/tauri/clipboard.ts` (12–14) e `features/clipboard-output/hooks/useClipboardOutput.ts` (34–47).
- **Responsabilidade atual:** um `File` é lido integralmente no frontend e convertido com `Array.from` para passar como `Vec<u8>` ao comando Rust.
- **Evidência:** o teto é 25 MB; converter `Uint8Array` em milhões de números cria uma representação adicional antes da serialização IPC.
- **Impacto:** maior uso de memória e latência para cópias locais próximas ao limite, e o mesmo padrão seria inadequado para uma biblioteca com arquivos duráveis.
- **Solução mínima:** ao implementar persistência P1, fazer a importação/gravação nativa receber um caminho permitido ou fluxo adequado ao Tauri, e fazer o clipboard referenciar o arquivo já persistido. Não otimizar prematuramente o fluxo transitório P0 isoladamente.
- **Risco da alteração:** médio; caminhos fornecidos pelo frontend exigem validação de escopo e cópia para diretório controlado.
- **Testes após a mudança:** arquivo próximo de 25 MB, falha de leitura/escrita, caminho fora do diretório controlado, preservação de bytes e cópia após reinício.

### C-1 — Estado técnico de importação e função pública não usados

- **Prioridade:** C — adiar sem prejuízo relevante.
- **Severidade:** baixa.
- **Localização:** `features/media-import/hooks/useMediaImport.ts` (34–113) e `features/media-import/lib/media.ts` (268–272).
- **Responsabilidade atual:** o hook mantém e retorna `technicalState`, mas nenhum consumidor o lê; `prepareMediaInput` apenas delega a `normalizeReceivedInput` e também não tem chamadas.
- **Evidência:** a busca de referências fora dos próprios módulos não encontra consumidores.
- **Impacto:** pequena superfície morta e dois conceitos de estado de importação que podem confundir a implementação da P1.
- **Solução mínima:** quando a P1 definir a tela de importação, escolher o estado que a UI realmente precisa e remover o outro com seus testes correspondentes. Não criar um store global para isso.
- **Risco da alteração:** baixo.
- **Testes após a mudança:** testes existentes de estado de importação e de normalização; verificar mensagens da nova tela de importação.

### C-2 — Documentação de arquitetura ainda descreve P0 como proposta

- **Prioridade:** C — atualizar antes ou durante a primeira entrega P1.
- **Severidade:** baixa.
- **Localização:** `docs/architecture.md` (cabeçalho e seções “P0.2”/“P0.3”) e `README.md` (“Estado atual”).
- **Responsabilidade atual:** documentos de orientação ainda usam linguagem de desenho/prova em trechos que antecedem o encerramento formal da P0.
- **Evidência:** `docs/p0-plan.md` registra P0 concluída e P1 autorizada, enquanto `docs/architecture.md` começa com “desenho ... proposto” e o README informa “Estamos na P0”.
- **Impacto:** novos contribuidores podem interpretar incorretamente o estado do produto e os limites das decisões já implementadas.
- **Solução mínima:** atualizar o status e distinguir decisões implementadas das que continuam futuras quando a especificação P1 for aprovada.
- **Risco da alteração:** baixo; é documentação, sem comportamento.
- **Testes após a mudança:** revisão cruzada com a matriz P0 e o plano P1 aprovado.

## 5. Riscos hipotéticos que exigem decisão, não correção imediata

| Risco | Evidência atual | Decisão necessária antes/durante P1 | Prioridade |
|---|---|---|---|
| Modelo de URLs temporárias/assinadas | P0 aceita HTTPS para prévia e download sob demanda; documentos já alertam que links podem expirar. | Definir se a biblioteca guarda URL original, cópia local, ambos, ou item indisponível recuperável. | A |
| Ciclo de vida de arquivos persistidos | O cache de clipboard é explicitamente temporário, com 24 h/100 MB, e não pode virar biblioteca. | Definir diretório controlado, nomenclatura, política de exclusão e relação entre arquivo persistido e cache CF_HDROP. | A |
| Busca e metadados | `GifSearch` usa exemplos estáticos; ainda não há modelo de título, tag, favorito ou coleção. | Definir invariantes do item e consultas que a primeira biblioteca precisa, antes de escolher esquema físico. | A |
| Acesso concorrente à biblioteca | O cache usa `CACHE_LOCK`; não há estado persistente ainda. | Escolher transação e serialização adequadas à biblioteca quando houver operações salvar/editar/excluir. | B |
| Compatibilidade Windows | Matriz P0 mantém Win10/Win11, WebView2 e falhas Win32 como validação manual. | Manter smoke tests nativos como gate de release; não inferir compatibilidade da suíte simulada. | B |

Esses são riscos de design futuro, não defeitos atuais: não há SQLite ou persistência para auditar nesta base.

## 6. Preparação para P1

### Pronto para reutilizar

- Validação de formato, tamanho e seleção em `media-import/lib/media.ts`.
- Contrato de entrada explícita em `shared/received-input.ts`.
- Adaptadores Tauri e o padrão de não expor detalhes nativos às features.
- Copiar link, download remoto protegido e publicação `CF_HDROP` como saídas independentes da futura busca.
- Preferências locais de tema/atalho e composição simples em `App`.

### Decisões que P1 deve tomar antes de escolher persistência

1. Qual é o identificador estável do item e quais campos pertencem ao item, à origem e ao arquivo local.
2. Quando uma URL remota é referência suficiente e quando a mídia precisa ser copiada para armazenamento controlado.
3. Como representar item indisponível sem apagar metadados, favoritos ou coleção.
4. Quais operações precisam ser atômicas entre arquivo no disco e índice de metadados.
5. Qual primeira experiência substitui a superfície diagnóstica para importar e salvar.

Essas decisões antecedem SQLite; elas evitam que o banco se torne o lugar onde detalhes temporários de UI são acidentalmente armazenados.

## 7. O que não deve ser refatorado agora

- **Não dividir `remote_media.rs` apenas por ter 620 linhas.** A fronteira transporte/orquestração já existe, a política de segurança fica próxima do uso e os 13 testes determinísticos exercitam o fluxo. Uma divisão sem uma nova responsabilidade aumentaria a distância de revisão de segurança.
- **Não dividir `clipboard.rs` apenas por ter 517 linhas.** Cache, proteção do `CF_HDROP`, publicação e limpeza participam da mesma transação lógica. Separar Win32 de cache só se uma nova interface concreta surgir na P1, preservando os testes de integridade.
- **Não criar repositório genérico, camada “service”, store global, pacote `core` ou abstração de banco antes do primeiro caso de uso persistente.** Não há reutilização comprovada que os justifique.
- **Não centralizar a validação TypeScript e Rust em uma falsa fonte única.** As validações duplicadas são intencionais em fronteiras de confiança diferentes: prévia no WebView e publicação/arquivo no processo nativo.
- **Não substituir o transporte reqwest/Rustls ou relaxar DNS/TLS para testes locais.** A abstração atual existe para testes determinísticos sem enfraquecer o caminho de produção.

## 8. Plano incremental recomendado

1. **Antes de iniciar o código P1 (A):** aprovar o contrato de item persistente e a política de fontes/arquivos; registrar a atualização de status dos documentos.
2. **Primeiro incremento P1 (A):** criar o fluxo mínimo “entrada validada → salvar item persistente → listar”, mantendo a prévia separada e sem introduzir coleções/favoritos antes de existir item salvo.
3. **Segundo incremento P1 (A):** substituir a UI diagnóstica pelo caso de uso de importação, preservando os classificadores e testes de entrada que continuam relevantes.
4. **Quando a cópia local for integrada à biblioteca (B):** eliminar a transferência de bytes grande pelo IPC em favor do arquivo persistido controlado; repetir testes de integridade e smoke test Windows.
5. **Conforme necessidade de produto (B):** adicionar metadados, favoritos, coleções e busca sobre o contrato já estabelecido.
6. **Higiene posterior (C):** remover estado/função sem consumidor e modernizar a narrativa P0 da documentação.

## Conclusão

**Recomendação: SIM COM AJUSTES.** A arquitetura atual está pronta para iniciar P1 desde que o primeiro passo seja definir e implementar a fronteira mínima de item persistente, em vez de conectar diretamente a prévia temporária ao futuro armazenamento. Não há evidência para reescrever downloader, cache, clipboard, adaptadores Tauri ou a organização atual por features antes disso.

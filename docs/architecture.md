# Arquitetura do GIFTray

> **Estado:** desenho de arquitetura proposto. A P0 verificará as principais hipóteses de integração com o Windows. Este documento descreve as decisões para pessoas que mantenham ou contribuam com o projeto.

## Limites do projeto

Há dois componentes no produto completo, com **repositórios e políticas de visibilidade distintos**:

1. **GIFTray desktop (este repositório, público e MPL-2.0):** pesquisa local, captura, organização, arquivos locais, cache temporário, clipboard, atalhos e integração com Windows 10/11.
2. **GIFTray platform (repositório separado e privado, futuro):** landing page, biblioteca web, autenticação, API, eventual sincronização de metadados e infraestrutura de operação.

A existência de uma plataforma privada não restringe o uso ou a redistribuição do código público nos termos da MPL-2.0. Não incluiremos código-fonte proprietário neste repositório.

## Monorepo público, com escopo deliberadamente pequeno

Usaremos **pnpm workspaces** para facilitar o desenvolvimento do frontend e de pacotes de reutilização real. Tauri usa **Cargo** para a parte Rust, que tem gerenciamento de dependências independente.

```text
GIFTray/                     # Este repositório público
├── apps/
│   └── desktop/             # Será gerado durante a P0
│       ├── src/             # React + TypeScript
│       └── src-tauri/       # Rust + APIs nativas
├── packages/                # Somente quando houver necessidade concreta
├── docs/                    # Documentação humana e evidências
├── README.md
├── CONTRIBUTING.md
├── AGENTS.md
├── LICENSE
├── package.json
└── pnpm-workspace.yaml

GIFTray Platform/            # Outro repositório, privado; não existe aqui
├── apps/web/
└── services/api/            # Local e organização a decidir na P2
```

O workspace está preparado mesmo com apenas uma aplicação. Não vamos criar `packages/core` por antecipação; lógica compartilhada deve surgir de reutilização comprovada. Não haverá `apps/web` no repositório público.

## Arquitetura interna do desktop (P0.2)

O primeiro spike começou com um único componente React. A P0.1.3 separa as responsabilidades que já têm comportamento próprio, sem criar estado global, wrappers de Fluent ou pacotes compartilhados especulativos.

```text
apps/desktop/src/
├── app/                         # composição, provider de tema e foco da janela
│   ├── App.tsx
│   ├── theme.ts
│   └── use-search-window-focus.ts
├── features/
│   ├── gif-search/              # busca local de exemplos e navegação por teclado
│   ├── input-diagnostics/       # drag/drop, colagem explícita e classificação segura
│   ├── media-import/            # normalização, seleção e prévia temporária de mídia
│   └── settings/                # preferências locais e diálogo de configurações
├── platform/tauri/              # contratos TypeScript com APIs Tauri
│   ├── input.ts
│   └── window.ts
├── main.tsx                     # bootstrap React
├── shared/                      # contrato de entrada usado por diagnóstico e importação
└── styles.css                   # regras globais mínimas e acessibilidade

apps/desktop/src-tauri/src/
├── lib.rs                       # montagem da aplicação e registro dos comandos
├── shortcuts.rs                 # estado e troca atômica do atalho global
└── window.rs                    # mostrar, focar e ocultar a janela
```

### Direção das dependências

`app` compõe features e escolhe o tema. Cada feature contém seu estado, UI e regras específicas. Em `input-diagnostics`, os componentes apresentam a zona de drop, a área de colagem e o resultado; o hook coordena estado e ciclos de vida DOM/Tauri; `diagnostics.ts` preserva a classificação pura e testável. A P0.2 acrescenta `media-import`: `lib/media.ts` normaliza as entradas e identifica assinaturas, o hook controla a prévia e libera object URLs, e os componentes apenas apresentam o resultado. O contrato `shared/received-input.ts` surgiu de reutilização concreta: o diagnóstico é o único capturador de eventos e entrega a entrada explicitamente recebida à importação, evitando listeners concorrentes. Uma feature pode chamar `platform/tauri` quando precisa de uma API nativa, mas `platform` não importa features nem regras de UI.

### Importação temporária de mídia

A P0.2 não grava arquivo, URL ou HTML. Em um `drop` ou em uma colagem explicitamente armada, o contrato compartilhado transporta arquivos e os campos `text/uri-list`, `text/plain` e `text/html` apenas para processamento em memória. O HTML nunca é inserido no DOM; somente valores de atributos `src` e `href` são considerados e passam pela validação de URL HTTPS.

Arquivos são limitados a 25 MB e precisam ter assinatura GIF, WebP ou PNG. GIF/WebP/PNG identificados por bytes são formatos confirmados; extensão da URL apenas infere formato. A animação é confirmada para WebP com indicador de animação e PNG com `acTL`, inferida para GIF com múltiplos descritores e URLs GIF/WebP, ou desconhecida nos demais casos. A seleção prefere animação confirmada, depois URL provavelmente animada, depois arquivo provavelmente animado e, por fim, mídia estática. Esse critério evita aceitar automaticamente uma miniatura estática quando há uma referência provável ao original.

Para arquivo local, a prévia usa object URL que é revogada ao limpar, substituir ou desmontar o componente. URL remota não é baixada por backend nem por proxy: a WebView tenta carregá-la diretamente e informa erro genérico se a origem expirar, bloquear CORS ou falhar.

No Rust, `lib.rs` apenas registra plugins, estado e comandos. `shortcuts.rs` preserva a regra de registrar o novo atalho antes de revogar o antigo. `window.rs` é o único local que manipula a janela principal e emite seu evento de foco. A interface React usa adaptadores tipados em `platform/tauri`, em vez de chamar `invoke` ou APIs de WebView diretamente.

### Como adicionar uma feature

1. Criar `features/<nome>/` somente para uma capacidade de produto com UI ou estado próprios.
2. Manter regras testáveis e seus testes junto da feature.
3. Expor uma integração Windows/Tauri nova por `platform/tauri` e manter o payload mínimo e tipado.
4. Compor a feature em `app/App.tsx`; não usar estado global se a comunicação puder ficar no pai imediato.
5. Criar um módulo Rust adicional somente quando um comando, estado ou ciclo de vida nativo passar a ter responsabilidade própria.

Essa organização preserva a P0 como experimento local: ela não cria biblioteca definitiva, armazenamento, sincronização ou contrato com a plataforma futura.

## Fluxo de dados da experiência central

```text
GIF externo/arquivo
      │ (arrastar, colar ou URL explícita)
      ▼
Captura e inspeção ── validação dos tipos ──┬── URL durável → referência local
                                          └── arquivo → mídia no disco local
                                                      │
                                                      ▼
                                            índice de metadados local
                                                      │
                                     atalho global + busca por nome/tags
                                                      │
                                                      ▼
                                    URL, arquivo original ou download
                                                      │
                                                      ▼
                                       colagem manual em outro app
```

Na P0, a indexação e o armazenamento permanente são experimentais. É suficiente demonstrar fluxo com itens de teste antes de adotar a base SQLite na P1.

## Tipos de itens e suas garantias

| Tipo | O que preservamos | Limitação |
|---|---|---|
| URL remota | Referência e metadados | Provedor pode trocar endereço, bloquear hotlink ou expirar assinatura |
| Arquivo local | Bytes no computador, metadados e origem quando existir | Mídia não se transfere automaticamente para outros dispositivos |
| Referência indisponível | Metadados e status recuperável | Não é possível copiar a mídia sem recuperar outra origem |

Não confundiremos uma miniatura visível com prova de que o original animado está disponível. Devemos reconhecer MIME real, tamanho e duração quando possível, e sinalizar links expirados.

## Integração nativa Windows

- **Janela:** compacta, rápida, com foco na busca; retorno do foco anterior após copiar ou fechar; não ocupar desnecessariamente a barra de tarefas.
- **Atalho global:** configurável, com sugestão inicial `Ctrl+Alt+G`, confirmação de registro e tratamento de colisões. Manter o atalho antigo caso a alteração falhe.
- **Tema:** seguir o sistema por padrão, permitindo Claro/Escuro manual. Visual Fluent em Win10 22H2 e Win11, fallback sólido sem Mica/Acrylic quando indisponível.
- **Clipboard e drag-and-drop:** Tauri fornece algumas APIs, mas preservação de animação e formatos recebidos precisam ser verificados por origem/destino. Integração Rust/Win32 pontual somente se necessária.
- **Resiliência:** biblioteca buscável offline; mídia remota pode exigir rede; downloads temporários e cache possuem limite de espaço e limpeza.

## Segurança e privacidade

Somente importação explícita. Não monitorar permanentemente a área de transferência nem enviar conteúdo privado para telemetria. Tratar HTML/URL/arquivos como entradas não confiáveis. Permitir protocolos e tipos esperados, limitar downloads, sanitizar logs e aplicar permissões Tauri mínimas. Nunca criar serviço que faça proxy indiscriminado de URLs arbitrárias fornecidas pelo usuário.

## Futuro contrato desktop–plataforma

Quando houver sincronização, o desktop usará uma API versionada e autenticada. Podem ser compartilhadas **especificações públicas de contrato**, não necessariamente código-fonte. Os dados sincronizados serão principalmente metadados; arquivos guardados somente em um PC terão indicação clara de indisponibilidade na web. A estratégia exata de conflitos e autenticação será definida na P2.

## Decisões, alternativas e justificativas

| Escolha | Por quê | Alternativa se não funcionar |
|---|---|---|
| Tauri 2 + React + TypeScript | Equilíbrio entre familiaridade de desenvolvimento e integração nativa | APIs Win32 específicas; reavaliar WinUI 3 se inviável |
| pnpm workspace sem orquestrador | Fácil compartilhar scripts e futuros pacotes sem custo de configuração elevado | Adicionar cache/orquestração apenas diante de gargalos reais |
| Repositório desktop público, plataforma privada | Possibilita contribuições no app mantendo independência operacional da plataforma | Reavaliar separação se contratos e releases crescerem em complexidade |
| Referência remota ou arquivo local | Evita storage em nuvem e impede perda imediata de arquivo recebido | Considerar storage opcional quando houver demanda/custo validado |
| Busca local | Evita latência por requisição e permite uso offline | Revisar índice caso haja escala incompatível com a implementação |

## Critérios para revisar esta arquitetura

A [P0](p0-plan.md) e sua [matriz de testes](p0-test-matrix.md) são as fontes de evidências. Mudanças significativas devem registrar o problema observado, opções analisadas e consequências técnicas, sem transformar esta página em um relatório de execução do agente.

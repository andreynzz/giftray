# P0 — Spike executável do desktop

> **Status:** implementação inicial; a interoperabilidade com aplicativos externos e a execução nos dois Windows ainda exigem evidência manual.

## Escopo entregue

`apps/desktop` concentra o primeiro executável: Tauri 2, React, TypeScript, Vite e Fluent UI React v9. A janela de busca usa dados locais de exemplo, navega com setas, recebe foco ao abrir e é ocultada por `Esc`.

O processo nativo registra `Ctrl+Alt+G` e pode atualizar o atalho por comando. A troca tenta registrar a nova combinação antes de remover a anterior; se o Windows a rejeitar, a antiga permanece ativa. A escolha de atalho e de tema é guardada no armazenamento local do WebView, sem conta, rede ou telemetria.

O tema Sistema acompanha `prefers-color-scheme`; Claro e Escuro são opções explícitas. A interface usa cores sólidas Fluent e indicadores de foco, com redução de movimento quando solicitada pelo sistema. Ela não depende de Mica ou Acrylic, permitindo um fallback visual previsível no Windows 10.

## Diagnóstico de entrada

A área experimental reage somente a arrastar/soltar ou `Ctrl+V` com foco nela. Ela mostra categorias de formato (`URL`, `HTML`, `arquivo`, `imagem`) a partir dos MIME types e metadados de `DataTransfer`; não mostra, registra, persiste ou envia o conteúdo importado. Este desenho existe para coletar evidências manuais sem expor URLs assinadas ou dados de clipboard.

## Limitações conhecidas e próximos testes manuais

- Confirmar em Windows 10 22H2 e Windows 11 o registro, conflito e persistência do atalho depois de reiniciar.
- Testar foco, DPI, alto contraste, tema em tempo real, múltiplos monitores e WebView2, preenchendo a matriz P0 com o ambiente real.
- Testar arrastar de navegador, Discord e Explorador, e a colagem explícita dos formatos previstos. Não há suporte declarado a clipboard animado ou drag-and-drop do Discord até obter essa evidência.
- Este spike classifica somente os tipos; ele não importa, armazena, copia ou reproduz GIFs e não prova preservação de animação.

## Decisões técnicas

- As preferências ficam no frontend porque a P0 não precisa de dados de biblioteca, banco ou sincronização.
- O registro de atalho e a janela ficam no Rust para usar a integração global do Tauri e preservar o atalho anterior em caso de conflito.
- Não foi criado pacote compartilhado: a lógica isolável é pequena e pertence a este app.

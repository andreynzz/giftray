# GIFTray — Pesquisa e diretrizes de UX/UI

**Status:** recomendação de design para revisão, não especificação implementada.
**Escopo:** interface Windows 10/11 em Tauri 2 + React + Fluent UI React v9, a partir da P1.1.3.
**Pesquisa:** outubro de 2026.
**Contexto técnico auditado:** código publicado em `develop` após PR #4; alterações locais posteriores à PR não foram inspecionadas.
**Objetivo central:** capturar sem interromper e reencontrar/copiar mídia animada com rapidez, sem tornar a organização obrigatória.

## 1. Tese de produto

O GIFTray combina dois trabalhos diferentes:

1. **Acesso rápido:** pessoa conversando em outro aplicativo invoca o atalho, reconhece o GIF e o copia para colar manualmente no destino.
2. **Gerenciamento:** pessoa importa arquivos, revisa a biblioteca e futuramente organiza favoritos, tags e coleções.

**Decisão proposta:** desenvolver progressivamente dois **contextos de uso**, sem impor duas janelas já na P1.1.3:

- **Painel rápido:** inspirado em lançadores Windows como PowerToys Command Palette e na navegação keyboard-first do Raycast. Busca/resultado/ação em primeiro plano; janela compacta; nenhuma ação de organização obrigatória.
- **Biblioteca visual:** inspirada em gerenciadores de assets como Eagle. Miniaturas, seleção, visualização e organização; pode futuramente usar janela separada ou modo expandido, depois de validar usabilidade, sem mudar a arquitetura agora.

**Regra de produto:** salvar e reutilizar são ações primárias; favoritar, classificar, renomear e agrupar são opcionais.

## 2. Referências e aprendizados (não copiar interfaces literalmente)

| Referência | Evidência documentada | Adotar | Evitar |
|---|---|---|---|
| Microsoft PowerToys Command Palette | Uso por atalho e teclado; modo de busca compacto que expande para resultados | Foco inicial claro, digitar imediatamente, densidade de informação | Transformar GIFTray em lançador genérico |
| Raycast Clipboard History | Entrada selecionada, Enter para ação principal, comandos adicionais pelo action panel, acesso via hotkey | Ação primária previsível, descoberta de atalhos, estados rápidos | Afirmar envio/colagem automática que GIFTray não executa |
| Eagle | Biblioteca de imagens, tags/pastas e busca visual | Grid para reconhecimento de imagens, organização progressiva | Painéis, ferramentas e filtros pesados na janela rápida |
| Windows Photos / Fluent | Conteúdo visual como foco, componentes familiares de Windows | Linguagem visual familiar, navegação consistente, espaçamento | Copiar aparência WinUI/XAML ao pé da letra no React |
| Nielsen Norman Group | Visibilidade de estado, consistência, prevenção de erro, reconhecimento em vez de memorização | Estados explícitos, mensagens e fluxos previsíveis | Modal para todo sucesso, interfaces cheias de instruções |

## 3. Diagnóstico do frontend publicado

**Constatações do código público, não do trabalho local não enviado:**

- `apps/desktop/src-tauri/tauri.conf.json`: janela principal **640×550**, mínimo **420×420**, `alwaysOnTop: true`, `skipTaskbar: true`, redimensionável.
- `apps/desktop/src/app/App.tsx`: empilha cabeçalho, busca de demonstração, diagnósticos, prévia transitória e saída de clipboard.
- `apps/desktop/src/features/gif-search/index.tsx`: busca usa três itens fictícios; setas alteram o índice de aparência; `role=listbox` envolve botões, sem semântica `option` apropriada nem ativação completa de resultado. É protótipo, não navegação final acessível.
- O tema é controlado por FluentProvider; configurações e atalho já funcionam. Preservar esses recursos.

**Implicações:** conteúdo P0 precisa ficar disponível para regressão, mas não deve competir visualmente com a biblioteca real. Não exibir resultados fictícios como itens reais. Refatorar a semântica de busca e teclado quando houver busca funcional (P1.3); não estender o protótipo.

## 4. Hierarquia de informação

### P1.1.3 — primeira biblioteca, SEM preview persistente

Uma única coluna vertical; a ação de importar deve ser evidente. O cabeçalho permanece compacto. Lista de itens reais, inicialmente sem imagens, pois backend de prévia virá em P1.2. Diagnósticos P0 podem ficar numa área secundária recolhida, claramente rotulada como ferramenta técnica, sem remover funcionalidade ou testes.

```text
┌──────────────────────────────────────────────────────────┐
│ GIFTray                                   [⚙]      [×]   │
│ Biblioteca                                [＋ Importar]  │
│ 3 arquivos salvos                                        │
│ ┌──────────────────────────────────────────────────────┐ │
│ │ [ícone GIF] reaction.gif           GIF · 2,4 MB     │ │
│ │ [ícone WebP] loop.webp            WebP · 846 KB     │ │
│ │ [ícone PNG] meme.png              PNG · 146 KB      │ │
│ └──────────────────────────────────────────────────────┘ │
│ [Ferramentas de teste da P0 ▾] [somente fase transição]   │
└──────────────────────────────────────────────────────────┘
```

Não prometer ação de copiar item persistido até P1.2. Sem thumbnails simuladas, botões sem comportamento ou busca falsa. Não adicionar barra lateral com três ou mais seções para biblioteca vazia.

### P1.2 — biblioteca visual e preview

Após o backend servir mídia por ID, considerar cards com **miniaturas estáticas** e prévia animada sob demanda/seleção. Em painel compacto, uma lista com miniatura pequena ajuda a navegar pelo teclado; para explorar uma galeria visual, um grid responsivo tem vantagem (guia Microsoft ListView vs GridView). Validar ambas as visões com tarefas reais.

```text
┌──────────────────────────────────────────────────────────┐
│ GIFTray                                  [＋]  [⚙]        │
│ [Pesquisar meus GIFs…                        Ctrl+F]      │
│ Recentes  ·  Favoritos (futuro)                          │
│ ┌───────────────┐ ┌───────────────┐ ┌───────────────┐   │
│ │  miniatura    │ │  miniatura    │ │  miniatura    │   │
│ │  imagem       │ │  imagem       │ │  imagem       │   │
│ └───────────────┘ └───────────────┘ └───────────────┘   │
│ reaction.gif          animação.webp       meme.png      │
│ [Selecionado: reaction.gif]          [Copiar arquivo]    │
└──────────────────────────────────────────────────────────┘
```

O wireframe é somente hipótese de interação. Busca real apenas na P1.3; não antecipar itens/abas inoperantes durante P1.2.

### Futura janela de gerenciamento

Se gestão avançada de centenas de itens tornar o painel compacto inadequado, estudar janela **não** `alwaysOnTop` e com presença na barra de tarefas, conservando o painel rápido `alwaysOnTop`. Essa decisão deve ser validada em teste real, não implementada antecipadamente.

## 5. Regras para mídia visual

- **Reconhecimento visual primeiro** assim que houver suporte de preview. Título é apoio, não único caminho de identificação.
- **Evitar tocar 20 GIFs ao mesmo tempo:** miniaturas estáticas por padrão; animação quando o usuário selecionar/focar/solicitar preview e possibilidade de parar. Respeitar `prefers-reduced-motion` e necessidade de pausar movimentação automática persistente (WCAG 2.2.2).
- Não supor que `<img>` permite pausar GIF arbitrariamente. Proposta futura: poster frame derivado, com GIF original preservado e exibido somente quando solicitado, mediante mecanismo técnico validado.
- Conter imagem no quadro sem cortar a informação importante (`object-fit: contain` como padrão candidato); testar transparência, imagens muito altas/largas, texto incorporado e fundos transparentes.
- Reservar espaço com proporção visual para evitar saltos de layout. Loading de mídia local não deve bloquear interface inteira.
- Se mídia estiver `missing`, mostrar sinalizador textual + ação disponível de ajuda/recuperação futura; não exibir thumbnail enganosa.
- Qualquer geração de thumbnail é derivada: **nunca alterar bytes originais** da biblioteca.
- Milhares de GIFs: carregamento por demanda, virtualização da lista/grid quando medido, não montar todos os elementos na memória.

## 6. Interação e teclado

| Contexto | Convenção proposta | Fase |
|---|---|---|
| Atalho global | Abre painel e coloca foco na busca quando ela for funcional; durante P1.1, foco previsível na ação principal ou lista | P1.1.3 → P1.3 |
| `Tab` / `Shift+Tab` | Percorrer controles em ordem visual, foco sempre perceptível | P1.1.3 |
| `Ctrl+O` | Abrir importação de arquivo (se não colidir com atalho existente) | P1.1.3 opcional |
| `↑` / `↓` | Navegar itens quando lista/results focados; teclado deve refletir seleção semântica e estado ARIA real | P1.1.3+ |
| `Enter` | Ativar o botão focado; na lista rápida, copiar item selecionado somente quando cópia existir | P1.1.3 → P1.2 |
| `Ctrl+F` | Focar busca real, nunca campo fictício | P1.3 |
| `Escape` | Fechar popover/dialog primeiro; limpar busca quando previsto; ocultar painel somente quando não houver camada superior | P1.1.3+ |
| Mouse | Hover mostra ações adicionais sem esconder ação primária ou tornar teclado dependente dele | P1.1.3+ |

Não emular Ctrl+V no aplicativo de destino sem contrato e consentimento próprios. Após copiar via `CF_HDROP`, mensagem correta: **“Arquivo copiado. Cole com Ctrl+V.”** Não dizer “enviado” ou “colado”.

## 7. Estados e microcopy

| Estado | Mensagem / orientação proposta | UX |
|---|---|---|
| Biblioteca vazia | “Sua biblioteca está vazia” + “Importe seu primeiro GIF, WebP ou PNG.” + botão **Importar arquivo** | Convite à próxima ação |
| Carregando | “Carregando biblioteca…” se duração justificar; spinner discreto; sem flashing | Não confundir espera com vazio |
| Importando | “Importando arquivo…” com progresso indeterminado se sem contagem real | Sem porcentagem fictícia |
| Importado | “Arquivo salvo na biblioteca.” | Feedback curto, sem modal |
| Duplicado | “Esse arquivo já está na biblioteca.” | Feedback neutro/positivo, não erro |
| Cancelado | Silencioso; restaurar foco do botão Importar | Não punir cancelamento |
| Arquivo inválido | “Não foi possível importar. Escolha um GIF, WebP ou PNG válido de até 25 MB.” | Próximo passo claro |
| Erro de acesso | “Não foi possível ler o arquivo. Verifique se ainda está disponível e tente novamente.” | Evitar paths/stack trace |
| Falha ao carregar lista | “Não foi possível abrir sua biblioteca.” + **Tentar novamente** | Não mostrar estado vazio enganoso |
| Item ausente | “Arquivo indisponível” associado ao item | Sem depender apenas da cor |
| Copiado na P1.2 | “Arquivo copiado. Cole com Ctrl+V no aplicativo desejado.” | Descrever ação real |
| Busca sem resultado na P1.3 | “Nenhum GIF encontrado para ‘…’” + ação de limpar busca | Diferente de biblioteca vazia |

Preferir mensagem **inline perto da ação** ou MessageBar para falha persistente; sucesso efêmero via toast/live region, sem interromper foco. Fluent recomenda que mensagens de erro bloqueantes tragam ação para resolver. Toast não deve ocultar informação que ainda será necessária.

## 8. Linguagem visual / design system

**Base:** Fluent UI React v9 + princípios de Windows 11 (Effortless, Calm, Familiar). Não buscar “futurismo” por sombras/glass aleatórios.

- **Fonte:** Segoe UI, respeitando pilha do Fluent/Windows; título 18–20 px, corpo 14–16 px, metadados 12–13 px; revisar escala e legibilidade em 125/150/200%.
- **Paleta:** superfície neutra, uma cor de marca/acento para ação primária e foco; usar tokens Fluent (`colorNeutralBackground*`, `colorNeutralForeground*`, `colorBrand*`, `colorStatus*`) em vez de hex fixo.
- **Densidade:** sistema de espaçamento aproximado de 4/8/12/16/24 px, adaptado a tokens; 12–16 px de padding externo na janela compacta; respiro consistente entre grupos.
- **Raio:** manter geometria do Fluent; 8–12 px em containers onde apropriado, não pilhas de cartões arredondados sem propósito.
- **Ícones:** ícones Fluent existentes; texto na ação primária e tooltip/aria-label nos botões apenas de ícone.
- **Ações:** no máximo uma ação primária visual forte por superfície. Menu overflow apenas para ações secundárias. Não encher cada item de botões permanentes.
- **Tema:** respeitar claro/escuro existente e alto contraste; não depender de Mica/Acrylic ou transparência de janela para parecer nativo.
- **Motion:** transições curtas, funcionais e respeitando preferência de movimento reduzido. Evitar loading infinito e efeitos em massa.
- **Larguras:** testar 420 px e 640 px de janela; a lista é layout seguro na menor largura; grid responsivo para biblioteca quando miniaturas existirem. Não presumir 3 colunas em 420 px.

## 9. Busca e organização (P1.3/P1.4)

- Buscar nos **itens salvos**, não em serviço externo. Nunca deixar o campo de demo P0 parecer funcional.
- Abrir por atalho com foco no campo de busca quando disponível; mostrar Recentes com query vazia; filtro digitado atualiza localmente.
- Ordenação determinística e previsível; indicar filtros ativos. Buscar por nome/título/alias/tags **somente à medida que campos existirem**.
- Estados distintos: biblioteca vazia, consulta sem resultados, erro de consulta, carregando.
- Favoritos e coleções são opcionais. Não exigir que o usuário nomeie um GIF ao salvá-lo.
- Em bibliotecas grandes, medir latência de pesquisa, virtualizar resultados e preservar seleção ao alterar lista.
- Ações em lote e sidebars complexas só quando testes com biblioteca real justificarem.

## 10. Qualidade e acessibilidade

Critérios verificáveis:

1. Funcionalidades primárias podem ser concluídas sem mouse; não há foco aprisionado e o indicador de foco é visível.
2. Combinação ARIA consistente: listas selecionáveis usam papéis apropriados e comportamento de roving tabIndex/aria-activedescendant quando aplicável; não misturar `role=listbox` com botões sem semântica coerente.
3. Textos normais com contraste mínimo de 4.5:1, controles/gráficos relevantes 3:1, seguindo WCAG; verificar estados hover/disabled/focus e tema escuro.
4. Nome acessível em controles de ícone; leitores de tela anunciam feedback importante com moderação.
5. Nenhum recurso essencial depende exclusivamente de hover, cor ou movimento.
6. Testes com 420×420 e 640×550, DPI 100%, 125%, 150% e 200%, tema escuro/claro e alto contraste.
7. Usuário pode cancelar seleção sem receber mensagem de falha; foco volta a contexto conhecido.
8. Erros não divulgam URLs com credenciais, nomes de diretórios privados ou stack trace.
9. GIFs potencialmente animados em loop têm estratégia para pausar/reduzir movimento quando entrarem na UI, sem alterar o original.
10. UI nunca declara sucesso antes do backend confirmar commit/cópia.

## 11. Pesquisa com usuários e critérios de aceite

Testar presencialmente ou em screen-sharing com 5–8 participantes representativos (número de teste exploratório, não prova estatística):

- **Tarefa A:** biblioteca vazia → adicionar arquivo válido → identificar sucesso.
- **Tarefa B:** importar o mesmo arquivo → entender duplicata sem confundir com erro.
- **Tarefa C:** fechar processo → reabrir → localizar item.
- **Tarefa D (P1.2):** localizar item, copiar e colar no Discord/Explorer; entender que GIFTray não envia automaticamente.
- **Tarefa E (P1.3):** encontrar um GIF entre 100 usando teclado; distinguir biblioteca vazia de busca sem resultados.

Registrar: conclusão sem ajuda, passos/cliques, tempo (medição observada, não promessa), erros, hesitações, confusão entre salvar/copiar, foco/atalhos, comentários livres. Incluir pessoas que não desenvolveram o aplicativo.

**Critérios práticos para P1.1.3:** um único botão Importar claramente distinguível; lista persistente não falsa; estados importado/duplicado/cancelado/erro corretos; janela mínima utilizável; teclado funcional; o fluxo P0 continua acessível sem competir com função principal.

**Critérios práticos para P1.2:** preview realmente animado quando solicitado e suportado; botão Copiar claro; arquivo colado animado onde o destino aceita; cache independente; miniaturas não geram carga indiscriminada.

## 12. Roadmap UX/UI

| Entrega | UX prioritária | Não introduzir ainda |
|---|---|---|
| P1.1.3 | Biblioteca real em lista, importar, estados e feedback, navegação por teclado, responsividade | Busca falsa, thumbnails falsos, copiar por ID |
| P1.2 | Prévia restrita por ID, controle de animação, copiar arquivo explicitamente, feedback correto | Coleções, muitas animações em paralelo |
| P1.3 | Busca local real, fluxo de captura de produto, drop/paste com affordances, resultados e estados | Redesign total sem testes |
| P1.4 | Favoritos, tags, coleções opcionais, talvez contexto expandido de gestão | Organização obrigatória na captura |

## 13. Fontes prioritárias

**Documentação oficial:**

- Microsoft — Windows 11 Design Principles: https://learn.microsoft.com/en-us/windows/apps/design/design-principles
- Microsoft — Windows Layout/Content basics: https://learn.microsoft.com/windows/uwp/design/basics/content-basics
- Microsoft — ListView vs GridView: https://learn.microsoft.com/en-us/windows/uwp/controls-and-patterns/listview-and-gridview
- Microsoft — Keyboard Accessibility: https://learn.microsoft.com/windows/apps/design/accessibility/keyboard-accessibility
- Microsoft — PowerToys Command Palette: https://learn.microsoft.com/en-us/windows/powertoys/command-palette/overview
- Fluent 2 — Layout: https://fluent2.microsoft.design/layout
- Fluent 2 — Typography: https://fluent2.microsoft.design/typography
- Fluent 2 — Color Tokens: https://fluent2.microsoft.design/color-tokens
- Fluent 2 — Button: https://fluent2.microsoft.design/components/web/react/core/button/usage/
- Fluent 2 — Card: https://fluent2.microsoft.design/components/web/react/core/card/usage/
- Fluent 2 — List: https://fluent2.microsoft.design/components/web/react/core/list/usage
- Fluent 2 — Searchbox: https://fluent2.microsoft.design/components/web/react/core/searchbox/usage
- Fluent 2 — Toast: https://fluent2.microsoft.design/components/web/react/core/toast/usage
- Fluent 2 — Messagebar: https://fluent2.microsoft.design/components/web/react/core/messagebar/usage
- Fluent 2 — Skeleton: https://fluent2.microsoft.design/components/web/react/core/skeleton/usage/
- Fluent 2 — Motion: https://fluent2.microsoft.design/motion
- W3C — WCAG Pause/Stop/Hide: https://www.w3.org/WAI/WCAG20/Understanding/pause-stop-hide.html
- W3C — Focus visible: https://www.w3.org/WAI/WCAG22/Understanding/focus-visible
- W3C — Motion from interactions: https://www.w3.org/WAI/WCAG22/Understanding/animation-from-interactions

**Pesquisa, heurísticas e produtos:**

- Nielsen Norman Group — Heurísticas: https://www.nngroup.com/articles/ten-usability-heuristics/
- Nielsen Norman Group — Empty states: https://www.nngroup.com/articles/empty-state-interface-design/
- Raycast — Clipboard History: https://manual.raycast.com/clipboard-history
- Raycast — Redesign de busca/actions: https://www.raycast.com/blog/a-fresh-look-and-feel
- Eagle — Features da biblioteca visual: https://www.eagle.cool/home

## 14. Nota de proveniência

As seções **2, 6, 7, 8, 10** utilizam orientações extraídas ou inspiradas das fontes citadas, adaptadas para o GIFTray; as seções **1, 4, 5, 9, 11, 12** são **recomendações de produto** e hipóteses a validar — não afirmações de que a Microsoft, NN/g, Raycast ou Eagle prescrevem esses exatos fluxos, dimensões ou tempos.

# Orientações para agentes de programação

Este arquivo é operacional. Para compreender o produto, leia os documentos escritos para pessoas em `README.md` e `docs/`.

## Antes de editar

- Leia o escopo da tarefa, a visão em `docs/product-vision.md` e a arquitetura em `docs/architecture.md` quando necessário.
- Se a tarefa for P0, consulte `docs/p0-plan.md` e `docs/p0-test-matrix.md`.
- Inspecione estado do Git e dependências antes de gerar arquivos. Não sobrescreva mudanças existentes nem tente reestruturar componentes fora do escopo.
- Planeje incrementos pequenos, com condições verificáveis e riscos explícitos.

## Engenharia

- Repositório **público desktop-only**: `apps/desktop` e, apenas se necessário, `packages/*`. Não crie `apps/web`, backend, API, autenticação ou código da plataforma privada aqui.
- `pnpm workspaces` para pacotes JS/TS e Cargo para Rust. Não introduza Nx, Turborepo, paid services, pacotes compartilhados especulativos ou dependências sem justificativa.
- Priorize separação de responsabilidades, código simples, validações e testes reproduzíveis.
- Recursos essenciais devem funcionar tanto em Windows 10 22H2 quanto Windows 11. Verifique degradês de aparência, tema, foco, teclado, DPI e fallback sem efeitos visuais avançados.
- Trate mídia arrastada, HTML, arquivos e URLs como dados não confiáveis; nunca execute conteúdo importado nem armazene tokens e URLs sensíveis em logs. Não monitore continuamente o clipboard; ler só por ação explícita.
- Não diga que clipboard animado, drag-and-drop do Discord ou compatibilidade Windows foi validada sem teste em ambiente real.
- Para arquivos-fonte próprios, adicione o cabeçalho MPL-2.0 no formato apropriado quando aplicável; não altere texto da licença e respeite licenças de terceiros.

## Git e integração

- Use Git Flow: `main` estável, `develop` integração, `feature/...` a partir de `develop`.
- Conventional Commits por responsabilidade. Não faça push, merge, tag, release ou deploy sem autorização explícita.
- Preserve histórico e não altere branches não relacionadas.

## Qualidade e entregáveis

- Registre testes automatizados executados e testes manuais ainda pendentes separadamente.
- Atualize `docs/` com decisões e evidências legíveis para humanos. Não converta `README.md` em log de implementação nem os docs em prompts para agentes.
- No resumo: mudança, arquivos relevantes, testes, restrições, commits e próximo experimento; seja transparente sobre o que não foi executado.

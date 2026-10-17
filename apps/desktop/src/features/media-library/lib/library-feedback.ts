// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

function errorDetail(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/** Maps the Rust boundary's already-sanitized errors to actionable UI copy. */
export function importFailureMessage(error: unknown): string {
  const detail = errorDetail(error);

  if (detail.includes("biblioteca está em uso")) {
    return "A biblioteca está em uso por outra instância. Feche a outra instância e tente novamente.";
  }

  if (detail.includes("abrir o arquivo selecionado")) {
    return "Não foi possível ler o arquivo. Verifique se ainda está disponível e tente novamente.";
  }

  if (detail.includes("formato GIF, WebP ou PNG confirmado")) {
    return "Não foi possível importar. Escolha um GIF, WebP ou PNG válido de até 25 MB.";
  }

  if (detail.includes("excede o limite de 25 MB")) {
    return "Não foi possível importar. Escolha um arquivo de até 25 MB.";
  }

  return "Não foi possível concluir a importação. Tente novamente.";
}

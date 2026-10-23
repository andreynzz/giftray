// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { describe, expect, it } from "vitest";
import { copyFailureMessage, importFailureMessage, renameFailureMessage } from "./library-feedback";

describe("importFailureMessage", () => {
  it("keeps expected boundary failures actionable without exposing details", () => {
    expect(importFailureMessage(new Error("Não foi possível abrir o arquivo selecionado."))).toContain("Verifique");
    expect(importFailureMessage("O arquivo excede o limite de 25 MB.")).toContain("25 MB");
    expect(importFailureMessage("A biblioteca está em uso por outra instância.")).toContain("outra instância");
  });

  it("uses a safe fallback for an unknown failure", () => {
    expect(importFailureMessage(new Error("internal sqlite path"))).toBe(
      "Não foi possível concluir a importação. Tente novamente.",
    );
  });
});

describe("copyFailureMessage", () => {
  it("maps unavailable and unknown failures to accessible messages", () => {
    expect(copyFailureMessage("Esse arquivo não está mais disponível na biblioteca.")).toContain("Atualize");
    expect(copyFailureMessage("private cache path")).toBe("Não foi possível copiar o arquivo. Tente novamente.");
  });
});

describe("renameFailureMessage", () => {
  it("maps expected rename failures without exposing internals", () => {
    expect(renameFailureMessage("O nome informado não é válido.")).toContain("120 caracteres");
    expect(renameFailureMessage("O arquivo solicitado não foi encontrado na biblioteca.")).toContain("Atualize");
    expect(renameFailureMessage("A biblioteca está em uso por outra instância.")).toContain("outra instância");
  });

  it("uses a rename-specific safe fallback", () => {
    expect(renameFailureMessage(new Error("sqlite database E:\\private\\library.db"))).toBe(
      "Não foi possível renomear o arquivo. Tente novamente.",
    );
  });
});

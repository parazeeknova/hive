import { readFileSync } from "node:fs";

import solc from "solc";
import type { Abi, Hex } from "viem";

interface CompilerOutput {
  errors?: { severity: string; formattedMessage: string }[];
  contracts?: Record<
    string,
    Record<string, { abi: Abi; evm: { bytecode: { object: string } } }>
  >;
}
export const compileToken = async () => {
  const source = await Bun.file(
    new URL("../../contracts/HiveTestUSDC.sol", import.meta.url)
  ).text();
  const output: CompilerOutput = JSON.parse(
    solc.compile(
      JSON.stringify({
        language: "Solidity",
        settings: {
          evmVersion: "cancun",
          optimizer: { enabled: true, runs: 200 },
          outputSelection: { "*": { "*": ["abi", "evm.bytecode.object"] } },
        },
        sources: { "HiveTestUSDC.sol": { content: source } },
      }),
      {
        import: (name: string) => {
          if (!name.startsWith("@openzeppelin/contracts/")) {
            return { error: "Unsupported import" };
          }
          try {
            return {
              contents: readFileSync(
                Bun.resolveSync(name, import.meta.dir),
                "utf-8"
              ),
            };
          } catch {
            return { error: `Missing Solidity dependency: ${name}` };
          }
        },
      }
    )
  );
  const errors = output.errors?.filter((error) => error.severity === "error");
  if (errors?.length) {
    throw new Error(errors.map((error) => error.formattedMessage).join("\n"));
  }
  const contract = output.contracts?.["HiveTestUSDC.sol"]?.HiveTestUSDC;
  if (!contract || !/^[\da-f]+$/iu.test(contract.evm.bytecode.object)) {
    throw new Error("Missing token bytecode");
  }
  return {
    abi: contract.abi,
    bytecode: `0x${contract.evm.bytecode.object}` as Hex,
  };
};

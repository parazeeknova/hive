"use client";
import { useState } from "react";

import { hiveway, errorMessage } from "../../lib/hiveway";
import { Json } from "./workspace-pages";

const operations = {
  artifact: {
    input: {
      input_schema: { type: "object" },
      output_schema: { type: "object" },
      wasm_base64: "",
    },
    label: "Upload Wasm artifact",
    path: "/v1/artifacts",
  },
  capability: {
    input: {
      description: "",
      name: "",
      price: "0",
      runtime: "repository_stack",
    },
    label: "Publish capability",
    path: "/v1/capabilities",
  },
  workflow: {
    input: { description: "", name: "", price: "0", steps: [] },
    label: "Compose workflow",
    path: "/v1/workflows",
  },
};
const CreatorTools = ({
  token,
  onRefresh,
}: {
  token: string;
  onRefresh: () => void;
}) => {
  const [operation, setOperation] =
    useState<keyof typeof operations>("capability");
  const [input, setInput] = useState(
    JSON.stringify(operations.capability.input, null, 2)
  );
  const [result, setResult] = useState<unknown>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  return (
    <form
      className="ws-form mt-[18px] flex flex-col items-stretch gap-[18px] max-[760px]:[&>.ws-button]:min-h-[44px]"
      onSubmit={async (event) => {
        event.preventDefault();
        setBusy(true);
        setError("");
        setResult(null);
        try {
          setResult(
            await hiveway(operations[operation].path, token, {
              body: JSON.stringify(JSON.parse(input) as unknown),
              method: "POST",
            })
          );
          onRefresh();
        } catch (error_) {
          setError(errorMessage(error_));
        }
        setBusy(false);
      }}
    >
      <h2 className="font-display">Create</h2>
      <label className="ws-field flex flex-col gap-[7px] text-[10px] [font-weight:650] text-[#667655] [&_input]:w-full [&_input]:min-w-0 [&_input]:rounded-[8px] [&_input]:bg-white [&_input]:px-[11px] [&_input]:py-2.5 [&_input]:text-[12px] [&_input]:font-normal [&_input]:text-[#4e5c40] [&_input]:[border:1px_solid_#dfe4d5] max-[760px]:[&_input]:text-[16px] [&_input::placeholder]:text-[#686f60] [&_select]:w-full [&_select]:min-w-0 [&_select]:rounded-[8px] [&_select]:bg-white [&_select]:px-[11px] [&_select]:py-2.5 [&_select]:text-[12px] [&_select]:font-normal [&_select]:text-[#4e5c40] [&_select]:[border:1px_solid_#dfe4d5] max-[760px]:[&_select]:text-[16px] [&_small]:text-[10px] [&_small]:leading-[1.7] [&_small]:font-normal [&_small]:text-[#686f60] [&_textarea]:w-full [&_textarea]:min-w-0 [&_textarea]:[resize:vertical] [&_textarea]:rounded-[8px] [&_textarea]:bg-white [&_textarea]:px-[11px] [&_textarea]:py-2.5 [&_textarea]:text-[12px] [&_textarea]:font-normal [&_textarea]:text-[#4e5c40] [&_textarea]:[border:1px_solid_#dfe4d5] max-[760px]:[&_textarea]:text-[16px] [&_textarea::placeholder]:text-[#686f60]">
        Operation
        <select
          value={operation}
          onChange={(event) => {
            const next = event.target.value as keyof typeof operations;
            setOperation(next);
            setInput(JSON.stringify(operations[next].input, null, 2));
            setResult(null);
          }}
          className="cursor-pointer"
        >
          {Object.entries(operations).map(([key, value]) => (
            <option key={key} value={key}>
              {value.label}
            </option>
          ))}
        </select>
      </label>
      <p>
        GitHub accounts can upload and publish free tools with price 0. Connect
        a wallet to publish paid tools. Prices use atomic USDC units (1 USDC =
        1000000). Wasm publishing requires runtime “wasm” and an artifact_id
        from a successful upload. Workflows accept up to four owned repository
        capability IDs in steps.
      </p>
      <label className="ws-field flex flex-col gap-[7px] text-[10px] [font-weight:650] text-[#667655] [&_input]:w-full [&_input]:min-w-0 [&_input]:rounded-[8px] [&_input]:bg-white [&_input]:px-[11px] [&_input]:py-2.5 [&_input]:text-[12px] [&_input]:font-normal [&_input]:text-[#4e5c40] [&_input]:[border:1px_solid_#dfe4d5] max-[760px]:[&_input]:text-[16px] [&_input::placeholder]:text-[#686f60] [&_select]:w-full [&_select]:min-w-0 [&_select]:rounded-[8px] [&_select]:bg-white [&_select]:px-[11px] [&_select]:py-2.5 [&_select]:text-[12px] [&_select]:font-normal [&_select]:text-[#4e5c40] [&_select]:[border:1px_solid_#dfe4d5] max-[760px]:[&_select]:text-[16px] [&_small]:text-[10px] [&_small]:leading-[1.7] [&_small]:font-normal [&_small]:text-[#686f60] [&_textarea]:w-full [&_textarea]:min-w-0 [&_textarea]:[resize:vertical] [&_textarea]:rounded-[8px] [&_textarea]:bg-white [&_textarea]:px-[11px] [&_textarea]:py-2.5 [&_textarea]:text-[12px] [&_textarea]:font-normal [&_textarea]:text-[#4e5c40] [&_textarea]:[border:1px_solid_#dfe4d5] max-[760px]:[&_textarea]:text-[16px] [&_textarea::placeholder]:text-[#686f60]">
        Definition JSON
        <textarea
          rows={12}
          required
          value={input}
          onChange={(event) => setInput(event.target.value)}
        />
      </label>
      <button
        className="ws-button inline-flex min-h-[38px] cursor-pointer items-center justify-center gap-[9px] rounded-[999px] bg-white px-[15px] py-[9px] text-center text-[11px] [font-weight:650] shadow-[inset_0_1px_0_#ffffff80,_0_1px_2px_#252a1b04] [border:1px_solid_#dedfd7] disabled:cursor-not-allowed disabled:opacity-[0.45] [&.ws-primary]:[border-color:#20231e] [&.ws-primary]:text-[#fff] [&.ws-primary]:shadow-[inset_0_1px_1px_#ffffff20] [&.ws-primary]:[background:linear-gradient(#353831,_#20231e)] [&.ws-small]:min-h-[33px] [&.ws-small]:px-3 [&.ws-small]:py-[7px] [&.ws-small]:text-[10px] [&:hover]:[border-color:#bbc1ae] [&:hover]:bg-[#f4f5ee]"
        type="submit"
        disabled={busy}
      >
        {busy ? "Submitting…" : operations[operation].label}
      </button>
      {error && <p role="alert">{error}</p>}
      {result !== null && <Json value={result} />}
    </form>
  );
};
export default CreatorTools;

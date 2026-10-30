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
      className="ws-form"
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
      <h2>Create</h2>
      <label className="ws-field">
        Operation
        <select
          value={operation}
          onChange={(event) => {
            const next = event.target.value as keyof typeof operations;
            setOperation(next);
            setInput(JSON.stringify(operations[next].input, null, 2));
            setResult(null);
          }}
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
      <label className="ws-field">
        Definition JSON
        <textarea
          rows={12}
          required
          value={input}
          onChange={(event) => setInput(event.target.value)}
        />
      </label>
      <button className="ws-button" type="submit" disabled={busy}>
        {busy ? "Submitting…" : operations[operation].label}
      </button>
      {error && <p role="alert">{error}</p>}
      {result !== null && <Json value={result} />}
    </form>
  );
};
export default CreatorTools;

"use client";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";

import { errorMessage, hiveway } from "../../lib/hiveway";
import { Json } from "./workspace-pages";

interface Build {
  id: string;
  state: string;
  request: { name: string };
  model: string;
  attempts: number;
  source: string | null;
  validation: unknown;
  capability_id: string | null;
  error_code: string | null;
}
const active = (build?: Build) =>
  build?.state === "queued" || build?.state === "generating";

const running = (builds?: Build[]) =>
  Boolean(builds?.some((build) => active(build)));

interface BuilderConfiguration {
  builder: { model: string; account_daily_limit: number } | null;
}
const BuilderProvider = ({
  configuration,
  error,
}: {
  configuration: BuilderConfiguration | undefined;
  error: unknown;
}) => (
  <>
    {configuration?.builder && (
      <p className="ws-fine mt-3 text-[10px] leading-[1.9] [overflow-wrap:anywhere] text-[#686f60]">
        Model: {configuration.builder.model}. Up to{" "}
        {configuration.builder.account_daily_limit} builds per account in 24
        hours, subject to the shared server limit.{" "}
        {configuration.builder.model.includes("contributor") &&
          "Meta Contributor may use your task, examples and generated code to train its models."}
      </p>
    )}
    {configuration && !configuration.builder && (
      <p>Hosted builds are not configured on this server.</p>
    )}
    {error && <p role="alert">{errorMessage(error)}</p>}
  </>
);
const HostedBuilder = ({ token }: { token: string }) => {
  const client = useQueryClient();
  const [id, setId] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [examples, setExamples] = useState(
    '[\n  { "input": { "n": 3 }, "expected": { "doubled": 6 } },\n  { "input": { "n": 7 }, "expected": { "doubled": 14 } }\n]'
  );
  const key = useRef<string | null>(null);
  const configuration = useQuery({
    queryFn: () => hiveway<BuilderConfiguration>("/api/config"),
    queryKey: ["builder-configuration", token],
  });
  const builds = useQuery({
    queryFn: () => hiveway<Build[]>("/v1/builds", token),
    queryKey: ["hosted-builds", token],
    refetchInterval: (query) =>
      query.state.data?.some((build) => active(build)) ? 2000 : false,
  });
  const selected = useQuery({
    enabled: Boolean(id),
    queryFn: () => hiveway<Build>(`/v1/builds/${id}`, token),
    queryKey: ["hosted-build", token, id],
    refetchInterval: (query) => (active(query.state.data) ? 2000 : false),
  });
  useEffect(() => {
    if (selected.data?.state === "published") {
      void client.invalidateQueries({ queryKey: ["hive"] });
      void client.invalidateQueries({ queryKey: ["hive-account", "plan"] });
    }
  }, [client, selected.data?.state]);
  return (
    <div className="ws-form mt-[18px] flex flex-col items-stretch gap-[18px] max-[760px]:[&>.ws-button]:min-h-[44px]">
      <h2 className="font-display">Build with Hive</h2>
      <p>
        Describe a reusable JSON tool and supply examples. Hive generates the
        code, compiles it, runs your examples and publishes it free when they
        pass. No wallet needed. Builds use shared daily limits.
      </p>
      <p className="ws-fine mt-3 text-[10px] leading-[1.9] [overflow-wrap:anywhere] text-[#686f60]">
        Hosted tools can transform and validate JSON. They cannot use the
        network, filesystem or external services. Your task and examples are
        sent to the configured model provider. Passing examples does not prove
        correctness for every input; review the source before relying on a tool.
      </p>
      <BuilderProvider
        configuration={configuration.data}
        error={configuration.error}
      />
      <form
        className="ws-form mt-[18px] flex flex-col items-stretch gap-[18px] max-[760px]:[&>.ws-button]:min-h-[44px]"
        onChange={() => {
          key.current = null;
        }}
        onSubmit={async (event) => {
          event.preventDefault();
          const form = new FormData(event.currentTarget);
          setBusy(true);
          setError("");
          try {
            if (!key.current) {
              key.current = crypto.randomUUID();
            }
            const build = await hiveway<Build>("/v1/builds", token, {
              body: JSON.stringify({
                description: form.get("description"),
                examples: JSON.parse(examples) as unknown,
                idempotency_key: key.current,
                name: form.get("name"),
                task: form.get("task"),
              }),
              method: "POST",
            });
            setId(build.id);
            void client.invalidateQueries({
              queryKey: ["hive-account", "plan"],
            });
            client.setQueryData(["hosted-build", token, build.id], build);
            await client.invalidateQueries({
              queryKey: ["hosted-builds", token],
            });
          } catch (error_) {
            setError(errorMessage(error_));
          }
          setBusy(false);
        }}
      >
        <label className="ws-field flex flex-col gap-[7px] text-[10px] [font-weight:650] text-[#667655] [&_input]:w-full [&_input]:min-w-0 [&_input]:rounded-[8px] [&_input]:bg-white [&_input]:px-[11px] [&_input]:py-2.5 [&_input]:text-[12px] [&_input]:font-normal [&_input]:text-[#4e5c40] [&_input]:[border:1px_solid_#dfe4d5] max-[760px]:[&_input]:text-[16px] [&_input::placeholder]:text-[#686f60] [&_select]:w-full [&_select]:min-w-0 [&_select]:rounded-[8px] [&_select]:bg-white [&_select]:px-[11px] [&_select]:py-2.5 [&_select]:text-[12px] [&_select]:font-normal [&_select]:text-[#4e5c40] [&_select]:[border:1px_solid_#dfe4d5] max-[760px]:[&_select]:text-[16px] [&_small]:text-[10px] [&_small]:leading-[1.7] [&_small]:font-normal [&_small]:text-[#686f60] [&_textarea]:w-full [&_textarea]:min-w-0 [&_textarea]:[resize:vertical] [&_textarea]:rounded-[8px] [&_textarea]:bg-white [&_textarea]:px-[11px] [&_textarea]:py-2.5 [&_textarea]:text-[12px] [&_textarea]:font-normal [&_textarea]:text-[#4e5c40] [&_textarea]:[border:1px_solid_#dfe4d5] max-[760px]:[&_textarea]:text-[16px] [&_textarea::placeholder]:text-[#686f60]">
          Name
          <input
            name="name"
            required
            maxLength={80}
            placeholder="Double an integer"
          />
        </label>
        <label className="ws-field flex flex-col gap-[7px] text-[10px] [font-weight:650] text-[#667655] [&_input]:w-full [&_input]:min-w-0 [&_input]:rounded-[8px] [&_input]:bg-white [&_input]:px-[11px] [&_input]:py-2.5 [&_input]:text-[12px] [&_input]:font-normal [&_input]:text-[#4e5c40] [&_input]:[border:1px_solid_#dfe4d5] max-[760px]:[&_input]:text-[16px] [&_input::placeholder]:text-[#686f60] [&_select]:w-full [&_select]:min-w-0 [&_select]:rounded-[8px] [&_select]:bg-white [&_select]:px-[11px] [&_select]:py-2.5 [&_select]:text-[12px] [&_select]:font-normal [&_select]:text-[#4e5c40] [&_select]:[border:1px_solid_#dfe4d5] max-[760px]:[&_select]:text-[16px] [&_small]:text-[10px] [&_small]:leading-[1.7] [&_small]:font-normal [&_small]:text-[#686f60] [&_textarea]:w-full [&_textarea]:min-w-0 [&_textarea]:[resize:vertical] [&_textarea]:rounded-[8px] [&_textarea]:bg-white [&_textarea]:px-[11px] [&_textarea]:py-2.5 [&_textarea]:text-[12px] [&_textarea]:font-normal [&_textarea]:text-[#4e5c40] [&_textarea]:[border:1px_solid_#dfe4d5] max-[760px]:[&_textarea]:text-[16px] [&_textarea::placeholder]:text-[#686f60]">
          Description
          <input
            name="description"
            required
            maxLength={1000}
            placeholder="Double the integer n in an input object"
          />
        </label>
        <label className="ws-field flex flex-col gap-[7px] text-[10px] [font-weight:650] text-[#667655] [&_input]:w-full [&_input]:min-w-0 [&_input]:rounded-[8px] [&_input]:bg-white [&_input]:px-[11px] [&_input]:py-2.5 [&_input]:text-[12px] [&_input]:font-normal [&_input]:text-[#4e5c40] [&_input]:[border:1px_solid_#dfe4d5] max-[760px]:[&_input]:text-[16px] [&_input::placeholder]:text-[#686f60] [&_select]:w-full [&_select]:min-w-0 [&_select]:rounded-[8px] [&_select]:bg-white [&_select]:px-[11px] [&_select]:py-2.5 [&_select]:text-[12px] [&_select]:font-normal [&_select]:text-[#4e5c40] [&_select]:[border:1px_solid_#dfe4d5] max-[760px]:[&_select]:text-[16px] [&_small]:text-[10px] [&_small]:leading-[1.7] [&_small]:font-normal [&_small]:text-[#686f60] [&_textarea]:w-full [&_textarea]:min-w-0 [&_textarea]:[resize:vertical] [&_textarea]:rounded-[8px] [&_textarea]:bg-white [&_textarea]:px-[11px] [&_textarea]:py-2.5 [&_textarea]:text-[12px] [&_textarea]:font-normal [&_textarea]:text-[#4e5c40] [&_textarea]:[border:1px_solid_#dfe4d5] max-[760px]:[&_textarea]:text-[16px] [&_textarea::placeholder]:text-[#686f60]">
          What should the tool do?
          <textarea
            name="task"
            required
            maxLength={4096}
            rows={4}
            placeholder="Accept an object with integer n between 0 and 1000000, no additional properties. Return an object with doubled equal to n times two. Handle any valid input in that range."
          />
        </label>
        <label className="ws-field flex flex-col gap-[7px] text-[10px] [font-weight:650] text-[#667655] [&_input]:w-full [&_input]:min-w-0 [&_input]:rounded-[8px] [&_input]:bg-white [&_input]:px-[11px] [&_input]:py-2.5 [&_input]:text-[12px] [&_input]:font-normal [&_input]:text-[#4e5c40] [&_input]:[border:1px_solid_#dfe4d5] max-[760px]:[&_input]:text-[16px] [&_input::placeholder]:text-[#686f60] [&_select]:w-full [&_select]:min-w-0 [&_select]:rounded-[8px] [&_select]:bg-white [&_select]:px-[11px] [&_select]:py-2.5 [&_select]:text-[12px] [&_select]:font-normal [&_select]:text-[#4e5c40] [&_select]:[border:1px_solid_#dfe4d5] max-[760px]:[&_select]:text-[16px] [&_small]:text-[10px] [&_small]:leading-[1.7] [&_small]:font-normal [&_small]:text-[#686f60] [&_textarea]:w-full [&_textarea]:min-w-0 [&_textarea]:[resize:vertical] [&_textarea]:rounded-[8px] [&_textarea]:bg-white [&_textarea]:px-[11px] [&_textarea]:py-2.5 [&_textarea]:text-[12px] [&_textarea]:font-normal [&_textarea]:text-[#4e5c40] [&_textarea]:[border:1px_solid_#dfe4d5] max-[760px]:[&_textarea]:text-[16px] [&_textarea::placeholder]:text-[#686f60]">
          Sample input objects and expected output objects
          <textarea
            rows={6}
            required
            value={examples}
            onChange={(event) => setExamples(event.target.value)}
          />
        </label>
        <button
          className="ws-button inline-flex min-h-[38px] cursor-pointer items-center justify-center gap-[9px] rounded-[999px] bg-white px-[15px] py-[9px] text-center text-[11px] [font-weight:650] shadow-[inset_0_1px_0_#ffffff80,_0_1px_2px_#252a1b04] [border:1px_solid_#dedfd7] disabled:cursor-not-allowed disabled:opacity-[0.45] [&.ws-primary]:[border-color:#20231e] [&.ws-primary]:text-[#fff] [&.ws-primary]:shadow-[inset_0_1px_1px_#ffffff20] [&.ws-primary]:[background:linear-gradient(#353831,_#20231e)] [&.ws-small]:min-h-[33px] [&.ws-small]:px-3 [&.ws-small]:py-[7px] [&.ws-small]:text-[10px] [&:hover]:[border-color:#bbc1ae] [&:hover]:bg-[#f4f5ee]"
          type="submit"
          disabled={
            busy || !configuration.data?.builder || running(builds.data)
          }
        >
          {busy ? "Queuing build…" : "Build and publish free"}
        </button>
        {error && <p role="alert">{error}</p>}
      </form>
      {builds.error && <p role="alert">{errorMessage(builds.error)}</p>}
      {builds.data?.map((build) => (
        <button
          className="ws-button inline-flex min-h-[38px] cursor-pointer items-center justify-center gap-[9px] rounded-[999px] bg-white px-[15px] py-[9px] text-center text-[11px] [font-weight:650] shadow-[inset_0_1px_0_#ffffff80,_0_1px_2px_#252a1b04] [border:1px_solid_#dedfd7] disabled:cursor-not-allowed disabled:opacity-[0.45] [&.ws-primary]:[border-color:#20231e] [&.ws-primary]:text-[#fff] [&.ws-primary]:shadow-[inset_0_1px_1px_#ffffff20] [&.ws-primary]:[background:linear-gradient(#353831,_#20231e)] [&.ws-small]:min-h-[33px] [&.ws-small]:px-3 [&.ws-small]:py-[7px] [&.ws-small]:text-[10px] [&:hover]:[border-color:#bbc1ae] [&:hover]:bg-[#f4f5ee]"
          type="button"
          key={build.id}
          onClick={() => setId(build.id)}
        >
          {build.request.name} · {build.state}
        </button>
      ))}
      {selected.error && <p role="alert">{errorMessage(selected.error)}</p>}
      {selected.data && (
        <div aria-live="polite">
          <p>
            {selected.data.state} · {selected.data.model} ·{" "}
            {selected.data.attempts} attempts
          </p>
          {selected.data.error_code && (
            <p role="alert">{selected.data.error_code}</p>
          )}
          {selected.data.capability_id && (
            <a
              href={`/dashboard?view=discover&q=${encodeURIComponent(selected.data.request.name)}`}
            >
              View published tool
            </a>
          )}
          {selected.data.validation !== null && (
            <Json value={selected.data.validation} />
          )}
          {selected.data.source && (
            <details>
              <summary className="cursor-pointer">Generated source</summary>
              <pre className="whitespace-pre-wrap">{selected.data.source}</pre>
            </details>
          )}
        </div>
      )}
    </div>
  );
};
export default HostedBuilder;

"use client";

import { useState } from "react";

import { Icon } from "../../components/icon";
import { toolIconBase, toolTint } from "../../components/tool-icon";
import { capabilities, money } from "../../lib/capabilities";

/* Class strings lifted from the former .button / .button-* rules.
   The leading space lets them be appended to an existing className. */
const page =
  "mx-auto max-w-6xl px-9 pt-10 pb-16 max-[760px]:px-5 max-[760px]:py-8";
const pageTitle =
  "font-display text-3xl font-semibold tracking-tight max-[760px]:text-2xl";
const pageLead = "mt-3 text-sm leading-relaxed text-stone-500";
const eyebrow =
  "mb-3.5 flex items-center gap-2 text-xs text-lime-700 max-[760px]:pl-1";
const notice =
  "my-6 flex items-start gap-2.5 rounded-xl border border-amber-200 bg-amber-50 px-4 py-3.5 text-xs leading-relaxed text-amber-800";
const noticeIcon = "mt-px shrink-0";
const panel =
  "mt-6 rounded-2xl border border-stone-200 bg-white p-6 shadow-xs max-[760px]:p-5";
const panelTitle = "mb-5 text-lg font-semibold tracking-tight";
const panelText = "text-xs leading-relaxed text-stone-500";
const field = "mb-5 flex flex-col gap-2 text-xs font-semibold text-stone-600";
const fieldControl =
  "w-full rounded-lg border border-stone-200 bg-stone-50 p-3 text-sm font-normal text-stone-700 focus:border-lime-600";
const fieldTextarea = `${fieldControl} min-h-24 resize-y`;
const fieldHelp = "text-xs font-normal text-stone-400";
const fieldGrid =
  "grid grid-cols-2 gap-4 max-[760px]:grid-cols-1 max-[760px]:gap-0";
const formActions = "flex flex-wrap items-center gap-3";
const savedMessage = "inline-flex items-center gap-1.5 text-xs text-lime-700";
const buttonBase =
  "inline-flex min-h-12 items-center justify-center gap-3 rounded-xl border border-transparent px-6 py-3 text-sm font-bold transition-[transform,box-shadow,background] duration-200 hover:-translate-y-0.5 active:translate-y-0";
const buttonDark =
  "bg-stone-800 text-white shadow-sm hover:bg-stone-700 hover:shadow-md";
const buttonLight = "border-stone-200 bg-white/70 shadow-xs hover:bg-white";
const buttonSmall = "min-h-10 rounded-lg px-4 py-2.5 text-sm";
const buttonDarkSmall = `${buttonBase} ${buttonDark} ${buttonSmall}`;
const buttonLightSmall = `${buttonBase} ${buttonLight} ${buttonSmall}`;
const buttonDarkIcon = `${buttonBase} ${buttonDark}`;
const iconButton =
  "inline-flex size-9 shrink-0 items-center justify-center rounded-lg border border-stone-200 bg-white hover:bg-stone-100";
const workflowOutput = "mt-5";
const workflowOutputPre =
  "my-3 max-h-96 overflow-y-auto rounded-lg border border-stone-200 bg-stone-50 p-4 text-xs leading-relaxed break-words whitespace-pre-wrap text-stone-700";

export interface Draft {
  id: string;
  name: string;
  description: string;
  type: string;
  price: number;
}
export interface WorkflowDraft {
  name: string;
  steps: string[];
}
export interface Execution {
  id: string;
  name: string;
  input: string;
  createdAt: string;
  output: Record<string, unknown>;
}

export const Composer = ({
  workflow: initialWorkflow,
  onChange,
}: {
  workflow: WorkflowDraft;
  onChange: (workflow: WorkflowDraft) => void;
}) => {
  const [workflow, setWorkflow] = useState(initialWorkflow);
  const [nextStep, setNextStep] = useState("docs-extractor");
  const [saved, setSaved] = useState(false);
  const [preview, setPreview] = useState(false);
  const steps = workflow.steps
    .map((id) => capabilities.find((item) => item.id === id))
    .filter((item) => item !== undefined);
  const update = (value: WorkflowDraft) => {
    setWorkflow(value);
    setSaved(false);
    setPreview(false);
  };
  return (
    <section className={page}>
      <span className={eyebrow}>A FEW GOOD TOOLS, WORKING TOGETHER</span>
      <h1 className={pageTitle}>Make the pieces click.</h1>
      <p className={pageLead}>
        Draft a workflow that the next agent can build on.
      </p>
      <div className={notice}>
        <Icon name="compose" size={16} className={noticeIcon} />
        This is a local workflow sketch. Live input mapping, execution, and
        publication will be part of the connected composer.
      </div>
      <div className="grid grid-cols-[1.1fr_1fr] gap-6 max-[1000px]:grid-cols-1">
        <div className={panel}>
          <h2 className={panelTitle}>Your workflow</h2>
          <label className={field}>
            Name
            <input
              className={fieldControl}
              value={workflow.name}
              maxLength={80}
              onChange={(event) =>
                update({ ...workflow, name: event.target.value })
              }
              placeholder="Give your workflow a name"
            />
          </label>
          <div className="flex flex-col gap-1.5">
            {steps.map((tool, index) => (
              <div key={`${tool.id}-${index}`}>
                {index > 0 && (
                  <div className="my-px ml-8 h-4 w-px bg-stone-200" />
                )}
                <div className="flex items-center gap-3 rounded-xl border border-stone-200 bg-stone-50 p-3">
                  <span className={`${toolIconBase} ${toolTint(tool.color)}`}>
                    <Icon name={tool.icon} size={18} />
                  </span>
                  <div className="flex flex-col gap-1">
                    <strong className="text-sm font-semibold">
                      {tool.name}
                    </strong>
                    <small className="text-xs text-stone-400">
                      Step {index + 1} · {money(tool.price)} / call
                    </small>
                  </div>
                  <button
                    type="button"
                    className={`${iconButton} ml-auto`}
                    aria-label={`Remove step ${index + 1}: ${tool.name}`}
                    onClick={() =>
                      update({
                        ...workflow,
                        steps: workflow.steps.filter(
                          (_, position) => position !== index
                        ),
                      })
                    }
                  >
                    <Icon name="close" size={13} />
                  </button>
                </div>
              </div>
            ))}
          </div>
          {steps.length === 0 && (
            <p className={fieldHelp}>
              Start with one capability. Add the next when you’re ready.
            </p>
          )}
          <div className="mt-4 flex gap-2">
            <select
              className="min-w-0 flex-1 rounded-lg border border-stone-200 bg-white p-2 text-xs text-stone-600"
              aria-label="Capability to add"
              value={nextStep}
              onChange={(event) => setNextStep(event.target.value)}
            >
              {capabilities.map((tool) => (
                <option key={tool.id} value={tool.id}>
                  {tool.name}
                </option>
              ))}
            </select>
            <button
              type="button"
              className={buttonLightSmall}
              onClick={() =>
                update({ ...workflow, steps: [...workflow.steps, nextStep] })
              }
            >
              <Icon name="plus" size={13} />
              Add step
            </button>
          </div>
          <div className="my-5 flex justify-between border-t border-stone-200 pt-4 text-xs text-stone-500">
            <span>Illustrative tool costs per run</span>
            <strong className="text-stone-600">
              {money(steps.reduce((sum, tool) => sum + tool.price, 0))}
            </strong>
          </div>
          <div className={formActions}>
            <button
              type="button"
              className={buttonDarkSmall}
              disabled={!workflow.name.trim() || steps.length === 0}
              onClick={() => {
                onChange(workflow);
                setSaved(true);
              }}
            >
              <Icon name="check" size={14} />
              Keep this draft
            </button>
            {saved && (
              <output className={savedMessage}>
                Draft kept in this workspace
              </output>
            )}
          </div>
        </div>
        <div className={panel}>
          <h2 className={panelTitle}>A preview of the handoff</h2>
          <p className={panelText}>
            Each step uses the same repository input. Output mapping and
            execution limits need to be configured before a workflow can be
            published.
          </p>
          <div className={notice}>
            <Icon name="file" size={15} className={noticeIcon} />
            <span>
              Input: repository_url + revision
              <br />
              Output: a collection of each tool’s structured response
            </span>
          </div>
          <button
            type="button"
            className={buttonLightSmall}
            disabled={steps.length === 0}
            onClick={() => setPreview(!preview)}
          >
            {preview ? "Hide" : "Show"} example outputs{" "}
            <Icon name="down" size={13} />
          </button>
          {preview && (
            <div className={workflowOutput}>
              <pre className={workflowOutputPre}>
                {JSON.stringify(
                  steps.map((tool, index) => ({
                    capability: tool.id,
                    sample_response: tool.output,
                    step: index + 1,
                  })),
                  null,
                  2
                )}
              </pre>
              <p className={fieldHelp}>
                Fixed examples · no workflow was executed
              </p>
            </div>
          )}
        </div>
      </div>
    </section>
  );
};

export const Publisher = ({ onSave }: { onSave: (draft: Draft) => void }) => {
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [type, setType] = useState("API");
  const [price, setPrice] = useState("0.01");
  const [saved, setSaved] = useState(false);
  return (
    <section className={page}>
      <span className={eyebrow}>BUILT SOMETHING USEFUL?</span>
      <h1 className={pageTitle}>Give it a life in the hive.</h1>
      <p className={pageLead}>
        Start with a clear promise: what goes in, what comes out, and what a
        call costs.
      </p>
      <div className={notice}>
        <Icon name="help" size={16} className={noticeIcon} />
        You can save a listing draft locally. Deployment, contract checks, and
        payments must be connected before it can go live.
      </div>
      <form
        className={panel}
        onSubmit={(event) => {
          event.preventDefault();
          if (!name.trim() || !description.trim()) {
            return;
          }
          onSave({
            description: description.trim(),
            id: crypto.randomUUID(),
            name: name.trim(),
            price: Number(price),
            type,
          });
          setSaved(true);
        }}
        onChange={() => setSaved(false)}
      >
        <h2 className={panelTitle}>The first little details</h2>
        <label className={field}>
          Capability name
          <input
            className={fieldControl}
            required
            maxLength={80}
            value={name}
            onChange={(event) => setName(event.target.value)}
            placeholder="Something clear, like Repository Analyzer"
          />
        </label>
        <label className={field}>
          What does it do?
          <textarea
            className={fieldTextarea}
            required
            maxLength={600}
            value={description}
            onChange={(event) => setDescription(event.target.value)}
            placeholder="Describe the job your capability does, and the result it returns."
          />
        </label>
        <div className={fieldGrid}>
          <label className={field}>
            Capability type
            <select
              className={fieldControl}
              value={type}
              onChange={(event) => setType(event.target.value)}
            >
              <option>API</option>
              <option>MCP Tool</option>
              <option>Workflow</option>
              <option>Agent</option>
            </select>
          </label>
          <label className={field}>
            Price per call (USD)
            <input
              className={fieldControl}
              type="number"
              required
              min="0"
              max="1000"
              step="0.01"
              value={price}
              onChange={(event) => setPrice(event.target.value)}
            />
            <span className={fieldHelp}>
              An illustrative price for your draft.
            </span>
          </label>
        </div>
        <div className={formActions}>
          <button type="submit" disabled={saved} className={buttonDarkIcon}>
            <Icon name={saved ? "check" : "plus"} size={16} />
            {saved ? "Draft saved" : "Save listing draft"}
          </button>
          {saved && (
            <output className={savedMessage}>
              You’ll find it in My tools.
            </output>
          )}
        </div>
      </form>
      <div className={panel}>
        <h2 className={panelTitle}>Before it takes flight</h2>
        <p className={panelText}>
          A published capability needs an implementation, input and output
          schemas, contract tests, resource limits, and a creator payout
          account. This preview saves the listing details so you have a place to
          start.
        </p>
      </div>
    </section>
  );
};

export const MyTools = ({
  drafts,
  onPublish,
  onDelete,
}: {
  drafts: Draft[];
  onPublish: () => void;
  onDelete: (id: string) => void;
}) => (
  <section className={page}>
    <span className={eyebrow}>YOUR CORNER OF THE HIVE</span>
    <h1 className={pageTitle}>Good things in the making.</h1>
    <p className={pageLead}>
      A home for the capabilities you’re getting ready to share.
    </p>
    <div className={notice}>
      <Icon name="box" size={16} className={noticeIcon} />
      Listing drafts are stored in this browser. They are private to this
      preview and have not been published.
    </div>
    <div className={panel}>
      <h2 className={panelTitle}>
        Your listing drafts{" "}
        <span className="rounded border border-amber-200 bg-amber-100 px-2 py-1 text-xs text-amber-800">
          {drafts.length}
        </span>
      </h2>
      {drafts.length ? (
        drafts.map((draft) => (
          <div
            className="flex items-center gap-3 border-b border-stone-200 py-4 first-of-type:pt-0"
            key={draft.id}
          >
            <span className={`${toolIconBase} bg-amber-50 text-amber-600`}>
              <Icon name="box" size={18} />
            </span>
            <div className="flex min-w-0 flex-col gap-1.5">
              <strong className="text-sm font-semibold">{draft.name}</strong>
              <small className="text-xs text-stone-400">
                {draft.type} · {money(draft.price)} / call · Draft
              </small>
              <small className="text-xs break-words text-stone-400">
                {draft.description}
              </small>
            </div>
            <button
              className={`${iconButton} ml-auto shrink-0`}
              type="button"
              aria-label={`Delete draft ${draft.name}`}
              onClick={() => onDelete(draft.id)}
            >
              <Icon name="close" size={14} />
            </button>
          </div>
        ))
      ) : (
        <div className="px-5 py-12 text-center text-stone-400">
          <Icon name="box" size={30} className="mx-auto mb-4" />
          <h2 className="mb-2.5 text-lg text-stone-600">
            Every useful thing starts somewhere.
          </h2>
          <p className="mb-5 text-sm leading-relaxed">
            Draft your first capability and make a little room for what’s next.
          </p>
        </div>
      )}
      <button
        type="button"
        className={buttonDarkSmall}
        onClick={onPublish}
        style={{ marginTop: 20 }}
      >
        <Icon name="plus" size={14} />
        Draft a capability
      </button>
    </div>
  </section>
);

export const Executions = ({
  executions,
  onExplore,
}: {
  executions: Execution[];
  onExplore: () => void;
}) => {
  const [expanded, setExpanded] = useState<string | null>(null);
  return (
    <section className={page}>
      <span className={eyebrow}>A LITTLE PAPER TRAIL</span>
      <h1 className={pageTitle}>See what came back.</h1>
      <p className={pageLead}>
        Inputs, outputs, and a clear record of each sample you try.
      </p>
      <div className={notice}>
        <Icon name="activity" size={16} className={noticeIcon} />
        These are sample previews from this session. No repository was fetched,
        no live capability ran, and no charges were made.
      </div>
      <div className={panel}>
        <h2 className={panelTitle}>Sample execution history</h2>
        {executions.length ? (
          <div className="overflow-x-auto">
            <table className="w-full border-collapse text-xs">
              <thead>
                <tr>
                  <th className="border-b border-stone-200 p-3 text-left text-xs font-medium text-stone-400">
                    Capability
                  </th>
                  <th className="border-b border-stone-200 p-3 text-left text-xs font-medium text-stone-400">
                    Time
                  </th>
                  <th className="border-b border-stone-200 p-3 text-left text-xs font-medium text-stone-400">
                    Result
                  </th>
                  <th className="border-b border-stone-200 p-3 text-left text-xs font-medium text-stone-400">
                    Charged
                  </th>
                  <th className="border-b border-stone-200 p-3 text-left text-xs font-medium text-stone-400">
                    <span className="sr-only">Details</span>
                  </th>
                </tr>
              </thead>
              <tbody>
                {executions.map((execution) => (
                  <tr key={execution.id}>
                    <td className="border-b border-stone-100 px-3 py-4 font-semibold text-stone-600">
                      {execution.name}
                    </td>
                    <td className="border-b border-stone-100 px-3 py-4 text-stone-600">
                      {new Date(execution.createdAt).toLocaleTimeString([], {
                        hour: "2-digit",
                        minute: "2-digit",
                      })}
                    </td>
                    <td className="border-b border-stone-100 px-3 py-4 text-stone-600">
                      Sample response
                    </td>
                    <td className="border-b border-stone-100 px-3 py-4 text-stone-600">
                      $0.00
                    </td>
                    <td className="border-b border-stone-100 px-3 py-4 text-stone-600">
                      <button
                        className={iconButton}
                        type="button"
                        aria-label={`View ${execution.name} sample response`}
                        aria-expanded={expanded === execution.id}
                        onClick={() =>
                          setExpanded(
                            expanded === execution.id ? null : execution.id
                          )
                        }
                      >
                        <Icon name="down" size={12} />
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
            {executions
              .filter((execution) => execution.id === expanded)
              .map((execution) => (
                <div className={workflowOutput} key={execution.id}>
                  <p className={fieldHelp}>Input: {execution.input}</p>
                  <pre className={workflowOutputPre}>
                    {JSON.stringify(execution.output, null, 2)}
                  </pre>
                </div>
              ))}
          </div>
        ) : (
          <div className="px-5 py-12 text-center text-stone-400">
            <Icon name="activity" size={30} className="mx-auto mb-4" />
            <h2 className="mb-2.5 text-lg text-stone-600">
              Quiet here, for now.
            </h2>
            <p className="mb-5 text-sm leading-relaxed">
              Try a capability’s sample input to start your history.
            </p>
            <button
              type="button"
              className={buttonLightSmall}
              onClick={onExplore}
            >
              Find a capability <Icon name="arrow" size={14} />
            </button>
          </div>
        )}
      </div>
    </section>
  );
};

export const Billing = ({
  budget,
  onSave,
}: {
  budget: number;
  onSave: (value: number) => void;
}) => {
  const [amount, setAmount] = useState(String(budget));
  const [saved, setSaved] = useState(false);
  return (
    <section className={page}>
      <span className={eyebrow}>A BUDGET WITH BOUNDARIES</span>
      <h1 className={pageTitle}>Keep the spending sensible.</h1>
      <p className={pageLead}>Your agent does the work. You set the limits.</p>
      <div className={notice}>
        <Icon name="wallet" size={16} className={noticeIcon} />
        These balances are illustrative. No wallet is connected, no funds are
        held, and changing the preview budget does not authorize payments.
      </div>
      <div className="mt-6 grid grid-cols-3 gap-4 max-[760px]:gap-2">
        <div className="rounded-2xl border border-stone-200 bg-white p-5 max-[760px]:px-2.5 max-[760px]:py-3.5">
          <span className="block text-xs text-stone-400">
            Example monthly spend
          </span>
          <strong className="my-3 block text-3xl font-medium text-stone-700 max-[760px]:text-2xl">
            $42.17
          </strong>
          <small className="text-xs text-stone-400">Illustrative balance</small>
        </div>
        <div className="rounded-2xl border border-stone-200 bg-white p-5 max-[760px]:px-2.5 max-[760px]:py-3.5">
          <span className="block text-xs text-stone-400">Preview budget</span>
          <strong className="my-3 block text-3xl font-medium text-stone-700 max-[760px]:text-2xl">
            {money(budget)}
          </strong>
          <small className="text-xs text-stone-400">Local preference</small>
        </div>
        <div className="rounded-2xl border border-stone-200 bg-white p-5 max-[760px]:px-2.5 max-[760px]:py-3.5">
          <span className="block text-xs text-stone-400">Example headroom</span>
          <strong className="my-3 block text-3xl font-medium text-stone-700 max-[760px]:text-2xl">
            {money(Math.max(0, budget - 42.17))}
          </strong>
          <small className="text-xs text-stone-400">
            Budget less example spend
          </small>
        </div>
      </div>
      <form
        className={panel}
        onSubmit={(event) => {
          event.preventDefault();
          onSave(Number(amount));
          setSaved(true);
        }}
      >
        <h2 className={panelTitle}>Your preview budget</h2>
        <label className={field}>
          Monthly limit (USD)
          <input
            className={fieldControl}
            type="number"
            min="0"
            max="100000"
            step="0.01"
            required
            value={amount}
            onChange={(event) => {
              setAmount(event.target.value);
              setSaved(false);
            }}
          />
          <span className={fieldHelp}>
            Used for the budget display in this browser.
          </span>
        </label>
        <div className={formActions}>
          <button className={buttonDarkSmall} type="submit">
            Save preview budget
          </button>
          {saved && (
            <output className={savedMessage}>
              <Icon name="check" size={13} />
              Budget updated
            </output>
          )}
        </div>
      </form>
      <div className={panel}>
        <h2 className={panelTitle}>A clear record, when payments go live.</h2>
        <p className={panelText}>
          Invocation charges, creator earnings, fees, and payout status will be
          tracked separately. A pending payment will stay distinct from a
          completed payout.
        </p>
      </div>
    </section>
  );
};

export const Settings = ({
  name,
  onSave,
}: {
  name: string;
  onSave: (value: string) => void;
}) => {
  const [value, setValue] = useState(name);
  const [saved, setSaved] = useState(false);
  return (
    <section className={page}>
      <span className={eyebrow}>MAKE YOURSELF AT HOME</span>
      <h1 className={pageTitle}>A little more you.</h1>
      <p className={pageLead}>
        The small details that make this your workspace.
      </p>
      <form
        className={panel}
        onSubmit={(event) => {
          event.preventDefault();
          if (value.trim()) {
            onSave(value.trim());
            setSaved(true);
          }
        }}
      >
        <h2 className={panelTitle}>Workspace preferences</h2>
        <label className={field}>
          Workspace name
          <input
            className={fieldControl}
            required
            maxLength={40}
            value={value}
            onChange={(event) => {
              setValue(event.target.value);
              setSaved(false);
            }}
          />
          <span className={fieldHelp}>Saved locally in this browser.</span>
        </label>
        <div className={formActions}>
          <button className={buttonDarkSmall} type="submit">
            Save changes
          </button>
          {saved && (
            <output className={savedMessage}>
              <Icon name="check" size={13} />
              Feels more like home.
            </output>
          )}
        </div>
      </form>
      <div className={panel}>
        <h2 className={panelTitle}>About this workspace</h2>
        <p className={panelText}>
          You’re exploring an early Hive product preview. Account sign-in, agent
          API keys, hosted execution, and payment settings will become available
          as the backend is connected. The preview does not require an account.
        </p>
      </div>
    </section>
  );
};

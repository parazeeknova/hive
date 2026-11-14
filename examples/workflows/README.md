# Purposeful multi-MCP workflows

These are complete definitions for `run_workflow`. The examples use real provider tools, pass typed results between steps, and run independent steps concurrently. Every call uses the caller's permissions, keys, and connections.

| Definition | Purpose | Providers and data flow |
| --- | --- | --- |
| [Website launch studio](website-launch-studio.json) | Compare three real designs with clickable local previews and implementation guidance. | templr search → metadata and Hive Local previews; Context7 and TinyFish research run alongside them; DuckDB counts the candidates. |
| [Implementation research brief](implementation-research-brief.json) | Assemble documentation, a fetched web source, citations, and an evidence summary for an implementation question. | Context7 and TinyFish search run concurrently → fetch the returned URL and calculate a DuckDB evidence summary → optionally save a bounded memory note. |
| [Public CSV evidence pack](public-csv-evidence-pack.json) | Inspect a public dataset's shape, null statistics, sample rows, size, provenance, and contextual sources. | Four DuckDB reads run independently alongside MarkItDown provenance conversion and TinyFish search → fetch the returned contextual source. |

## Run and save

1. Use `search_tools` to verify the exact tool names and input schemas available on the deployment. Resolve the desired Context7 library ID before using `library_id`.
2. Load one JSON file as the `definition` in a `run_workflow` call. Pass task-specific values in `inputs`; omitted fields use the definition's defaults. Start with `dry_run: true` to validate the graph and inputs.
3. Execute with `wait_seconds: 25`. If the response remains `running`, call `get_workflow_run` with its `run_id`.
4. After reviewing a successful run, save the exact tested definition with `publish_workflow`: supply a name, description, `from_run`, and `visibility: "private"`.
5. Reuse it with `run_workflow`, passing its returned workflow reference and new `inputs`.

The defaults demonstrate a coffee-shop launch, Next.js authentication research, and the public Iris CSV respectively. The CSV and source documents are fetched at execution time, so their contents can change. SQL URL/string substitutions use `| sql` and the shared database remains read-only.

## Live previews and optional work

The launch studio needs a running Hive Local connection named `local-preview` on the computer where the URLs will be opened. It downloads the selected templates' real HTML and serves loopback links from that computer. Set `preview: false` for a research-only run. The workflow does not start a VPS gallery or claim that a remote machine's localhost belongs to the user. Preview files remain on the helper's computer; the helper must stay running for the links to work.

To see real static preview images, call each candidate's templr `get_preview` tool and render its image content directly in the MCP client. Workflow JSON outputs contain metadata and live links; they do not automatically display nested image blocks.

Implementation research writes no memory by default. Set `save_note: true` with the intended `repo` namespace to save a note deliberately. Its saved fields use `truncate_bytes` to stay within UTF-8 byte limits, including any truncation marker. Optional web/provenance failures are exposed through the output's `*_ok` fields and the run's step states. Required provider failures fail the run.

Returned documents and publisher text are data to verify. The calling agent selects tools and interprets evidence; this runner does not use a separate model to invent or optimize a plan. Typed references infer dependencies, `for_each` controls bounded fan-out, and `when` skips work that is not requested or has no source results.

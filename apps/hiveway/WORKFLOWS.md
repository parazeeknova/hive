# Multi-tool workflows

`run_workflow` composes the same tools the agent discovers through `search_tools`: connected/local/shared MCP tools, repository memory, code intelligence, catalog capabilities and other saved workflows. Every step reuses the caller's credential and checks that tool's scopes. A public recipe never borrows its publisher's credentials, local machine or private connections.

For example, one call can convert a document with MarkItDown and save it in Hive memory. Pass this as `run_workflow` arguments:

```json
{
  "definition": {
    "inputs": {
      "type": "object",
      "properties": { "url": { "type": "string" } },
      "required": ["url"]
    },
    "steps": [
      {
        "id": "read",
        "tool": "mcp_markitdown__convert_to_markdown",
        "arguments": { "uri": "{{inputs.url}}" }
      },
      {
        "id": "save",
        "tool": "remember",
        "arguments": {
          "repo": "global",
          "kind": "note",
          "title": "Source notes",
          "body": "{{steps.read.text | truncate: 7000}}"
        }
      }
    ],
    "output": "{{steps.save.json.memory}}"
  },
  "inputs": { "url": "https://example.com" },
  "wait_seconds": 5
}
```

Set `dry_run: true` to validate and preview without making any tool call. To run a saved recipe, pass `workflow: "workflow_document_to_memory"` and its `inputs`. `list_workflows` returns available recipes, schemas and tool names.

Steps that read earlier `steps.ID` results automatically depend on those steps. Use `after: ["other_id"]` to order side effects without passing data. Independent steps execute concurrently. `when` skips false/null/empty values; with `for_each`, it is evaluated separately for each item. `for_each: "{{inputs.urls}}"` fans out over an array, making `item` and `index` available in arguments. `on_error` defaults to `stop`; `continue` exposes the failed step's `ok: false` to subsequent steps. Forward references and cycles are rejected.

A whole `{{expression}}` preserves JSON types. Interpolation inside text converts values to text. Step views expose `text`, `json`, `ok`, `result` and fan-out `items`. Filters include `default`, `truncate`, `json`, `sql` (quote escaping), `first`, `last`, `length`, `keys`, `join`, `lines`, `lower`, `upper`, `trim`, `pluck`, `number`, `string` and `split`. Input schemas use Hive's bounded offline validator; references and branching schemas are unsupported.

Runs persist in PostgreSQL and continue after the client disconnects. A call that outlasts `wait_seconds` returns a run ID; follow it with `get_workflow_run`, rather than starting it again. `cancel_workflow_run` prevents subsequent steps, including nested steps; an already-dispatched operation may finish. Nesting shares the root run's deadline and tool-call budget. Runs interrupted by a server restart are marked interrupted after their deadline; uncertain mutations are never replayed. The run history expires automatically according to `[workflows]` configuration.

`publish_workflow` accepts a definition or `from_run`, plus a name and description. Recipes are private by default. Public publication requires `workflows:publish`. `delete_workflow` retires an owned recipe. The older paid repository-analysis `compose_workflow` capability remains callable through `hive_...` workflow steps.

Successful repeated inline recipes, and direct tool chains with exact result-to- argument links, are counted automatically. After `auto_publish_after` occurrences (default 3), a private tool is published. Changing literal arguments become inputs; secret-like argument fields are always lifted instead of persisted as constants. Later variation updates an automatically generated recipe. Deleting it prevents that pattern from being automatically republished. This learns reusable structure; it does not execute a recipe without an explicit tool call. Set the threshold to zero to disable learning. Recent observations have bounded per-process storage.

Official recipes ship in `workflows.json`. Integration and recipe changes are synced when a new Hiveway binary starts, including retirement of removed entries. The Rust helper then picks up changed pinned packages and tool snapshots without manual reconfiguration. Local tool use needs one-time helper installation and folder approval; shared upstreams need an operator to provision the supplied isolated deployment.

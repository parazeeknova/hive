import type { IconName } from "../components/icon";

export type CapabilityType = "API" | "MCP Tool" | "Workflow" | "Agent";
export interface Capability {
  id: string;
  name: string;
  description: string;
  type: CapabilityType;
  tags: string[];
  price: number;
  seconds: number;
  verified: boolean;
  icon: IconName;
  color: string;
  provider: string;
  dependencies: string[];
  input: string;
  output: Record<string, unknown>;
}

export const capabilities: Capability[] = [
  {
    color: "sage",
    dependencies: ["Repository Analyzer", "SAST Scan"],
    description:
      "A second pair of eyes for your repository. Find security, dependency, and code quality issues in one pass.",
    icon: "shield",
    id: "repo-audit",
    input: "repository_url",
    name: "Repo Audit",
    output: {
      findings: [
        {
          file: "package.json",
          message: "Review two outdated dependencies.",
          severity: "medium",
        },
      ],
      repository: "hive-labs/example",
      revision: "main",
      status: "sample_complete",
      summary: { critical: 0, high: 0, medium: 2 },
    },
    price: 0.12,
    provider: "hive-labs",
    seconds: 30,
    tags: ["Security", "GitHub"],
    type: "Workflow",
    verified: true,
  },
  {
    color: "lavender",
    dependencies: [],
    description:
      "Turn a GitHub repository into a structured map of its stack, manifests, and dependencies.",
    icon: "git",
    id: "repository-analyzer",
    input: "repository_url",
    name: "Repository Analyzer",
    output: {
      databases: ["PostgreSQL"],
      evidence: ["Cargo.toml", "package.json"],
      frameworks: ["Axum", "Next.js"],
      languages: ["Rust", "TypeScript"],
    },
    price: 0.08,
    provider: "hive-labs",
    seconds: 20,
    tags: ["GitHub", "Analysis"],
    type: "MCP Tool",
    verified: true,
  },
  {
    color: "peach",
    dependencies: [],
    description:
      "Catch common security issues in source code with static analysis.",
    icon: "code",
    id: "sast-scan",
    input: "repository_url",
    name: "SAST Scan",
    output: { findings: [], ruleset: "sample-security", scanned_files: 42 },
    price: 0.06,
    provider: "codegarden",
    seconds: 15,
    tags: ["Security", "Code"],
    type: "API",
    verified: true,
  },
  {
    color: "blue",
    dependencies: [],
    description:
      "Check your dependency tree for known vulnerabilities and outdated packages.",
    icon: "box",
    id: "dependency-check",
    input: "repository_url",
    name: "Dependency Check",
    output: { dependencies: 28, outdated: 2, vulnerabilities: [] },
    price: 0.04,
    provider: "codegarden",
    seconds: 10,
    tags: ["Security", "Dependencies"],
    type: "API",
    verified: true,
  },
  {
    color: "butter",
    dependencies: [],
    description:
      "Give your agent the right context. Extract clean, structured documentation from a repository.",
    icon: "book",
    id: "docs-extractor",
    input: "repository_url",
    name: "Docs Extractor",
    output: {
      documents: [{ path: "README.md", title: "Getting started" }],
      format: "markdown",
    },
    price: 0.03,
    provider: "opencomb",
    seconds: 8,
    tags: ["GitHub", "Documentation"],
    type: "MCP Tool",
    verified: true,
  },
  {
    color: "rose",
    dependencies: [],
    description:
      "Understand the licenses in your project before they become a release-day surprise.",
    icon: "file",
    id: "license-checker",
    input: "repository_url",
    name: "License Checker",
    output: {
      conflicts: [],
      licenses: ["MIT", "Apache-2.0"],
      review_required: false,
    },
    price: 0.04,
    provider: "opencomb",
    seconds: 12,
    tags: ["Compliance", "Legal"],
    type: "API",
    verified: true,
  },
  {
    color: "lavender",
    dependencies: [],
    description:
      "A focused review with practical suggestions for clearer, more maintainable code.",
    icon: "layers",
    id: "code-review",
    input: "repository_url",
    name: "Code Quality Review",
    output: {
      suggestions: [
        {
          file: "src/index.ts",
          suggestion: "Extract repeated validation into a shared function.",
        },
      ],
    },
    price: 0.1,
    provider: "little-tools",
    seconds: 25,
    tags: ["Code Review", "GitHub"],
    type: "Agent",
    verified: false,
  },
  {
    color: "sage",
    dependencies: ["Repository Analyzer", "Docs Extractor"],
    description:
      "Map the work between your current stack and your next one, with a step-by-step migration report.",
    icon: "compose",
    id: "migration-report",
    input: "repository_url",
    name: "Migration Planner",
    output: {
      status: "sample_plan",
      steps: [
        "Inventory dependencies",
        "Review compatibility",
        "Update and validate",
      ],
    },
    price: 0.18,
    provider: "little-tools",
    seconds: 40,
    tags: ["Migration", "GitHub"],
    type: "Workflow",
    verified: false,
  },
];

export const money = (amount: number) => `$${amount.toFixed(2)}`;

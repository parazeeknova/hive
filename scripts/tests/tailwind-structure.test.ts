import { expect, test } from "bun:test";
import path from "node:path";

const nectar = path.resolve(import.meta.dir, "../../apps/nectar");

test("Nectar keeps only the shared Tailwind theme and animation stylesheet", async () => {
  const files: string[] = [];
  for await (const file of new Bun.Glob("**/*.css").scan({
    cwd: nectar,
    onlyFiles: true,
  })) {
    if (!file.startsWith(".next/") && !file.startsWith("node_modules/")) {
      files.push(file);
    }
  }
  expect(files.toSorted()).toEqual(["app/globals.css"]);
  const css = await Bun.file(path.join(nectar, "app/globals.css")).text();
  expect(css).toContain('@import "tailwindcss"');
  expect(css).not.toMatch(/@utility\s|\.(?:ws|scene|hive)-[\w-]+\s*[{:.]/u);
});

test("Nectar components use Tailwind instead of inline styles or deprecated form events", async () => {
  for await (const file of new Bun.Glob("**/*.{tsx,ts}").scan({
    cwd: nectar,
    onlyFiles: true,
  })) {
    if (
      file.startsWith(".next/") ||
      file.startsWith("node_modules/") ||
      file.endsWith(".d.ts")
    ) {
      continue;
    }
    const source = await Bun.file(path.join(nectar, file)).text();
    expect(source, file).not.toMatch(/\bstyle\s*=\s*[{"']/u);
    expect(source, file).not.toMatch(/\bFormEvent(?:Handler)?\b/u);
  }
});

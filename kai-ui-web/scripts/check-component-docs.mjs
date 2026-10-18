import { execSync } from "node:child_process";
import { existsSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const root = new URL("..", import.meta.url).pathname;
const componentsDir = join(root, "src/components");

const folders = readdirSync(componentsDir).filter((name) =>
  statSync(join(componentsDir, name)).isDirectory(),
);

const errors = [];

for (const folder of folders) {
  const readme = join(componentsDir, folder, "README.md");
  if (!existsSync(readme)) {
    errors.push(`Falta src/components/${folder}/README.md`);
  }
}

const changed = diffNames();
const byFolder = new Map();

for (const file of changed) {
  const match = file.match(/^src\/components\/([^/]+)\/(.+)$/);
  if (!match) continue;
  const [, folder, rest] = match;
  if (!byFolder.has(folder)) byFolder.set(folder, new Set());
  byFolder.get(folder).add(rest);
}

for (const [folder, files] of byFolder) {
  const codeChanged = [...files].some((file) => /\.(ts|tsx|css)$/.test(file));
  const readmeChanged = files.has("README.md");
  if (codeChanged && !readmeChanged) {
    errors.push(
      `src/components/${folder}/ cambió sin actualizar src/components/${folder}/README.md`,
    );
  }
}

if (errors.length > 0) {
  for (const error of errors) console.error(error);
  process.exit(1);
}

console.log(`docs: ${folders.length} componentes con README`);

function diffNames() {
  const baseRef = process.env.GITHUB_BASE_REF;
  const range = baseRef ? `origin/${baseRef}...HEAD` : "HEAD~1...HEAD";
  try {
    const output = execSync(`git diff --name-only ${range}`, {
      cwd: root,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    });
    return output.split("\n").map((line) => line.trim()).filter(Boolean);
  } catch {
    return [];
  }
}

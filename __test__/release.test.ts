/// <reference types="node" />

import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, test } from "vite-plus/test";

// Native Bash is available on Ubuntu and macOS. The release matrix exercises
// the actual smoke script on Windows using Git Bash.
describe.skipIf(process.platform === "win32")("release smoke failure propagation", () => {
  test.each([
    { cjs: 17, esm: 0, expected: 1 },
    { cjs: 0, esm: 17, expected: 1 },
    { cjs: 0, esm: 0, expected: 0 },
  ])("CJS $cjs and ESM $esm produce exit $expected", ({ cjs, esm, expected }) => {
    const directory = mkdtempSync(join(tmpdir(), "notify-status-smoke-"));
    try {
      const child = spawnSync(
        "bash",
        [
          "-c",
          `
            npm() { return 0; }
            sleep() { :; }
            node() { if [ "$1" = check.cjs ]; then return ${cjs}; else return ${esm}; fi; }
            source "$1" "1.1.2"
          `,
          "smoke-test",
          fileURLToPath(new URL("../.github/scripts/smoke-package.sh", import.meta.url)),
        ],
        {
          cwd: directory,
          encoding: "utf8",
          timeout: 10_000,
          env: { ...process.env, TMPDIR: directory },
        },
      );
      expect(child.error).toBeUndefined();
      expect(child.status, child.stderr).toBe(expected);
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  });
});

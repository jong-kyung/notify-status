/// <reference types="node" />

import { spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { describe, expect, test } from "vite-plus/test";

import { version } from "../package.json";

// Only the optional package is present, so a local .node cannot bypass the
// version check. Platform overrides and environment changes stay in the child.
describe.each([
  { platform: "darwin", arch: "arm64", suffix: "darwin-arm64" },
  { platform: "darwin", arch: "x64", suffix: "darwin-x64" },
  { platform: "win32", arch: "arm64", suffix: "win32-arm64-msvc" },
  { platform: "win32", arch: "x64", suffix: "win32-x64-msvc" },
])("native loader ($suffix)", ({ platform, arch, suffix }) => {
  test.each(["matching", "mismatched"])("%s package version with strict checks", (match) => {
    const directory = mkdtempSync(join(tmpdir(), "notify-status-binding-"));
    const nativeVersion = match === "matching" ? version : "0.0.0";
    try {
      copyFileSync(new URL("../binding.cjs", import.meta.url), join(directory, "binding.cjs"));
      const name = `notify-status-${suffix}`;
      const dependency = join(directory, "node_modules", name);
      mkdirSync(dependency, { recursive: true });
      writeFileSync(
        join(dependency, "package.json"),
        JSON.stringify({ name, version: nativeVersion, main: "index.cjs" }),
      );
      writeFileSync(
        join(dependency, "index.cjs"),
        "module.exports = { getNotificationStatus() {} };\n",
      );
      const child = spawnSync(
        process.execPath,
        [
          "-e",
          `
            Object.defineProperty(process, 'platform', { value: '${platform}' });
            Object.defineProperty(process, 'arch', { value: '${arch}' });
            const binding = require('./binding.cjs');
            require('node:assert/strict').equal(typeof binding.getNotificationStatus, 'function');
          `,
        ],
        {
          cwd: directory,
          encoding: "utf8",
          timeout: 10_000,
          env: {
            ...process.env,
            NAPI_RS_ENFORCE_VERSION_CHECK: "1",
            NAPI_RS_NATIVE_LIBRARY_PATH: "",
            NAPI_RS_FORCE_WASI: "",
            NAPI_RS_WASI_FLAVOR: "",
          },
        },
      );
      expect(child.error).toBeUndefined();
      expect(child.status, child.stderr).toBe(match === "matching" ? 0 : 1);
      if (match === "mismatched") {
        expect(child.stderr).toContain(
          `Native binding package version mismatch, expected ${version} but got ${nativeVersion}`,
        );
      }
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  });
});

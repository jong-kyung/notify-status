import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, test } from "vite-plus/test";

const TARGETS = ["darwin-arm64", "darwin-x64", "win32-arm64-msvc", "win32-x64-msvc"];

function runScript(
  script: string,
  argument: string,
  stubs: string,
  env: Record<string, string> = {},
) {
  const directory = mkdtempSync(join(tmpdir(), "notify-status-release-"));
  try {
    writeFileSync(
      join(directory, "package.json"),
      JSON.stringify({ name: "notify-status", version: "1.1.2" }),
    );
    for (const target of TARGETS) {
      const dependency = join(directory, "npm", target);
      mkdirSync(dependency, { recursive: true });
      writeFileSync(
        join(dependency, "package.json"),
        JSON.stringify({ name: `notify-status-${target}`, version: "1.1.2" }),
      );
    }
    for (const file of ["output", "environment", "calls"]) {
      writeFileSync(join(directory, file), "");
    }
    const child = spawnSync(
      "bash",
      [
        "-c",
        `${stubs}\nsource "$1" "$2"`,
        "release-test",
        fileURLToPath(new URL(`../.github/scripts/${script}.sh`, import.meta.url)),
        argument,
      ],
      {
        cwd: directory,
        encoding: "utf8",
        timeout: 10_000,
        env: {
          ...process.env,
          ...env,
          RUNNER_TEMP: directory,
          TMPDIR: directory,
          GITHUB_OUTPUT: join(directory, "output"),
          GITHUB_ENV: join(directory, "environment"),
          CALLS: join(directory, "calls"),
        },
      },
    );
    expect(child.error).toBeUndefined();
    return {
      ...child,
      output: readFileSync(join(directory, "output"), "utf8"),
      environment: readFileSync(join(directory, "environment"), "utf8"),
      calls: readFileSync(join(directory, "calls"), "utf8").split("\n").filter(Boolean),
    };
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
}

// Native Bash is available on Ubuntu and macOS. The release matrix exercises
// the actual smoke script on Windows using Git Bash.
describe.skipIf(process.platform === "win32")("release scripts", () => {
  test.each([
    { previous: "1.1.2", registry: "error", changed: false, status: 0 },
    { previous: "1.1.1", registry: "published", changed: false, status: 0 },
    { previous: "1.1.1", registry: "empty", changed: true, status: 0 },
    { previous: "1.1.1", registry: "missing", changed: true, status: 0 },
    { previous: "", registry: "missing", changed: true, status: 0 },
    { previous: "1.1.1", registry: "error", changed: null, status: 1 },
  ])(
    "check release: previous=$previous registry=$registry",
    ({ previous, registry, changed, status }) => {
      const result = runScript(
        "check-release",
        "previous-push-sha",
        `
        git() {
          echo "git:$*" >> "$CALLS"
          if [ "$1" = fetch ]; then [ -n "$PREVIOUS" ];
          else printf '{"version":"%s"}' "$PREVIOUS"; fi
        }
        npm() {
          echo "npm:$*" >> "$CALLS"
          case "$REGISTRY" in
            published) echo 1.1.2 ;;
            empty) return 0 ;;
            missing) echo E404 >&2; return 1 ;;
            *) echo E503 >&2; return 1 ;;
          esac
        }
      `,
        { PREVIOUS: previous, REGISTRY: registry },
      );
      expect(result.status, result.stderr).toBe(status);
      expect(result.output).toBe(
        `version=1.1.2\n${changed === null ? "" : `changed=${changed}\n`}`,
      );
      if (previous) expect(result.calls).toContain("git:show previous-push-sha:package.json");
      if (previous === "1.1.2") {
        expect(result.calls.some((call) => call.startsWith("npm:"))).toBe(false);
      }
    },
  );

  test.each([
    { version: "1.1.2", current: "", newest: "", scenario: "fresh", status: 0, count: 5 },
    {
      version: "1.1.2",
      current: "1.1.1",
      newest: "1.1.2",
      scenario: "resume",
      status: 0,
      count: 4,
    },
    { version: "1.1.2-beta.1", current: "", newest: "", scenario: "fresh", status: 0, count: 5 },
    { version: "1.1.2", current: "1.1.2", newest: "1.1.2", scenario: "fresh", status: 1, count: 0 },
    { version: "1.1.2", current: "1.1.3", newest: "1.1.3", scenario: "fresh", status: 1, count: 0 },
    { version: "1.1.2", current: "", newest: "", scenario: "tag-error", status: 1, count: 0 },
    { version: "1.1.2", current: "", newest: "", scenario: "prepare-error", status: 19, count: 0 },
    { version: "1.1.2", current: "", newest: "", scenario: "lookup-error", status: 1, count: 0 },
    { version: "1.1.2", current: "", newest: "", scenario: "publish-error", status: 19, count: 1 },
  ])(
    "publish: $scenario version=$version current=$current",
    ({ version, current, newest, scenario, status, count }) => {
      const result = runScript(
        "publish-packages",
        version,
        `
        npm() {
          if [ "$1" = view ]; then
            case "$2" in
              notify-status@latest|notify-status@next)
                if [ "$SCENARIO" = tag-error ]; then echo E503 >&2; return 1; fi
                echo "$CURRENT" ;;
              *)
                if [ "$SCENARIO" = lookup-error ]; then echo E503 >&2; return 1; fi
                if [ "$SCENARIO" = resume ] && [ "$2" = "notify-status-darwin-arm64@$VERSION" ]; then
                  echo "$VERSION"
                else echo E404 >&2; return 1; fi ;;
            esac
          elif [ "$1" = publish ]; then
            echo "publish:$(node -p "require('./package.json').name"):$*" >> "$CALLS"
            if [ "$SCENARIO" = publish-error ]; then return 19; fi
          else return 99; fi
        }
        npx() { printf '%s\n' "$CURRENT" "$NEWEST"; }
        vp() {
          echo "prepare:$*" >> "$CALLS"
          if [ "$SCENARIO" = prepare-error ]; then return 19; fi
        }
      `,
        { VERSION: version, CURRENT: current, NEWEST: newest, SCENARIO: scenario },
      );
      expect(result.status, result.stderr).toBe(status);
      const tag = version.includes("-") ? "next" : "latest";
      expect(result.environment).toBe(`tag=${tag}\n`);
      const published = result.calls.filter((call) => call.startsWith("publish:"));
      expect(published).toHaveLength(count);
      if (status === 0) {
        expect(result.calls[0]).toBe(
          "prepare:exec napi prepublish -t npm --no-gh-release --skip-optional-publish",
        );
        const targets = scenario === "resume" ? TARGETS.slice(1) : TARGETS;
        expect(published).toEqual([
          ...targets.map(
            (target) => `publish:notify-status-${target}:publish --access public --tag ${tag}`,
          ),
          `publish:notify-status:publish --access public --tag ${tag} --ignore-scripts`,
        ]);
      } else {
        expect(published.some((call) => call.startsWith("publish:notify-status:"))).toBe(false);
      }
    },
  );

  test.each([
    { cjs: 17, esm: 0, scenario: "steady", status: 1, attempts: 5 },
    { cjs: 0, esm: 17, scenario: "steady", status: 1, attempts: 5 },
    { cjs: 0, esm: 0, scenario: "steady", status: 0, attempts: 1 },
    { cjs: 0, esm: 0, scenario: "retry", status: 0, attempts: 2 },
    { cjs: 0, esm: 0, scenario: "bundled", status: 1, attempts: 5 },
  ])("smoke: $scenario CJS=$cjs ESM=$esm", ({ cjs, esm, scenario, status, attempts }) => {
    const result = runScript(
      "smoke-package",
      "1.1.2",
      `
        npm() {
          if [ "$1" = install ]; then
            echo "install:$2" >> "$CALLS"
            if [ "$SCENARIO" = bundled ]; then
              mkdir -p node_modules/notify-status
              touch node_modules/notify-status/notify-status.darwin-arm64.node
            fi
          fi
          return 0
        }
        node() {
          echo "$1" >> "$CALLS"
          if [ "$SCENARIO" = retry ] && [ "$(grep -c '^install:' "$CALLS")" = 1 ]; then return 17; fi
          if [ "$1" = check.cjs ]; then return "$CJS"; else return "$ESM"; fi
        }
        sleep() { echo "sleep:$1" >> "$CALLS"; }
      `,
      { CJS: String(cjs), ESM: String(esm), SCENARIO: scenario },
    );
    expect(result.status, result.stderr).toBe(status);
    expect(result.calls.filter((call) => call === "install:notify-status@1.1.2")).toHaveLength(
      attempts,
    );
    expect(result.calls.filter((call) => call === "sleep:20")).toHaveLength(
      status === 0 ? attempts - 1 : attempts,
    );
    if (cjs !== 0 || scenario === "bundled") expect(result.calls).not.toContain("check.mjs");
  });
});

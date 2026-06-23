import { describe, expect, test } from "vite-plus/test";

import { getNotificationStatus } from "../src/index.js";

const isLinux = process.platform === "linux";
const linuxTest = isLinux ? test : test.skip;

// Covers AE5 / R8 — Linux is intentionally not supported. The library must
// return `{ authorization: 'unsupported', reason: 'unsupportedPlatform' }`
// and never throw. This guarantees safe import in Electron apps that ship
// a Linux build even though the feature is a no-op there.
describe("Linux — AE5 (unsupported platform)", () => {
  linuxTest("returns unsupported(unsupportedPlatform) without crashing", async () => {
    const status = await getNotificationStatus();
    expect(status.platform).toBe("linux");
    expect(status.authorization).toBe("unsupported");
    expect(status.doNotDisturb).toBe(false);
    expect(status.reason).toBe("unsupportedPlatform");
  });

  linuxTest("the function never throws", async () => {
    await expect(getNotificationStatus()).resolves.toBeDefined();
  });

  linuxTest("ten concurrent calls all resolve identically", async () => {
    const results = await Promise.all(Array.from({ length: 10 }, () => getNotificationStatus()));
    for (const r of results) {
      expect(r.platform).toBe("linux");
      expect(r.authorization).toBe("unsupported");
      expect(r.reason).toBe("unsupportedPlatform");
    }
  });
});

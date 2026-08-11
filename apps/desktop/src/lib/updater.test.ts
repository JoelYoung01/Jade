import { describe, expect, it } from "vitest";

import {
  buildLinuxRoutes,
  formatBytes,
  isNewerVersion,
  parseSemver,
  progressPercent,
  type InstallContext,
} from "@/lib/updater";

const baseCtx = (overrides: Partial<InstallContext> = {}): InstallContext => ({
  kind: "unknown",
  platform: "linux",
  distroId: "endeavouros",
  archBased: true,
  packageName: null,
  konsoleAvailable: true,
  yayAvailable: true,
  appImageEnv: null,
  releasesUrl: "https://github.com/JoelYoung01/Jade/releases/latest",
  ...overrides,
});

describe("parseSemver / isNewerVersion", () => {
  it("parses plain and v-prefixed versions", () => {
    expect(parseSemver("0.1.1")).toEqual([0, 1, 1]);
    expect(parseSemver("v0.2.0")).toEqual([0, 2, 0]);
  });

  it("compares versions", () => {
    expect(isNewerVersion("0.1.0", "0.1.1")).toBe(true);
    expect(isNewerVersion("0.1.1", "0.1.1")).toBe(false);
    expect(isNewerVersion("0.2.0", "0.1.9")).toBe(false);
  });
});

describe("progressPercent / formatBytes", () => {
  it("returns null while the download size is unknown", () => {
    expect(
      progressPercent({ phase: "downloading", downloaded: 512, contentLength: null }),
    ).toBeNull();
  });

  it("reports download percent and pins the install phase at 100", () => {
    expect(
      progressPercent({ phase: "downloading", downloaded: 50, contentLength: 200 }),
    ).toBe(25);
    expect(
      progressPercent({ phase: "installing", downloaded: 50, contentLength: 200 }),
    ).toBe(100);
  });

  it("formats byte counts", () => {
    expect(formatBytes(900)).toBe("900 B");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(12 * 1024 * 1024)).toBe("12 MB");
  });
});

describe("buildLinuxRoutes", () => {
  it("recommends AppImage for the current AppImage install", () => {
    const routes = buildLinuxRoutes({
      ctx: baseCtx({ kind: "appImage", appImageEnv: "/tmp/Jade.AppImage" }),
      remoteVersion: "0.1.2",
      aurRemoteVersion: "0.1.2",
      aurFetchFailed: false,
    });
    expect(routes[0]?.id).toBe("appImage");
    expect(routes[0]?.recommendedLabel).toBe("Recommended (current install)");
    expect(routes[0]?.action).toBe("installAppImage");
    expect(routes.find((r) => r.id === "aur")?.recommended).toBe(false);
  });

  it("recommends AUR for jade-desktop-bin installs and disables without yay", () => {
    const routes = buildLinuxRoutes({
      ctx: baseCtx({
        kind: "aur",
        packageName: "jade-desktop-bin",
        yayAvailable: false,
      }),
      remoteVersion: "0.1.2",
      aurRemoteVersion: "0.1.2",
      aurFetchFailed: false,
    });
    const aur = routes.find((r) => r.id === "aur");
    expect(aur?.recommendedLabel).toBe("Recommended (current install)");
    expect(aur?.enabled).toBe(false);
    expect(aur?.disabledReason).toMatch(/yay/i);
  });

  it("hides AUR route on non-Arch Linux", () => {
    const routes = buildLinuxRoutes({
      ctx: baseCtx({
        kind: "deb",
        archBased: false,
        distroId: "ubuntu",
      }),
      remoteVersion: "0.1.2",
      aurRemoteVersion: null,
      aurFetchFailed: false,
    });
    expect(routes.map((r) => r.id)).toEqual(["appImage"]);
    expect(routes[0]?.recommendedLabel).toBe("Recommended");
    expect(routes[0]?.action).toBe("downloadAppImage");
  });
});

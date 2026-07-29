import { describe, expect, it } from "vitest";

import {
  buildLinuxRoutes,
  isNewerVersion,
  parseSemver,
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

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock, openMock, saveMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  openMock: vi.fn(),
  saveMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: (...args: unknown[]) => openMock(...args),
  save: (...args: unknown[]) => saveMock(...args),
}));

import type { ImportSummary } from "../../portable";
import { PortableSection } from "../SettingsView";

function summary(patch: Partial<ImportSummary> = {}): ImportSummary {
  return {
    profileCount: 2,
    activeProfile: "imported",
    lightTheme: "latte",
    darkTheme: "mocha",
    keymaps: [
      { profile: "imported", status: "embedded", valid: true },
      { profile: "spare", status: "none", valid: true },
    ],
    deviceResetToAuto: true,
    ...patch,
  };
}

beforeEach(() => {
  invokeMock.mockReset();
  openMock.mockReset();
  saveMock.mockReset();
});

afterEach(() => {
  cleanup();
});

describe("PortableSection", () => {
  it("renders export and import actions", () => {
    render(<PortableSection reload={() => {}} />);
    expect(screen.getByText("export JSON")).toBeTruthy();
    expect(screen.getByText("import JSON")).toBeTruthy();
  });

  it("shows a validated replacement preview for a valid import", async () => {
    openMock.mockResolvedValue("/tmp/backup.json");
    invokeMock.mockImplementation((cmd: string) =>
      cmd === "inspect_import" ? Promise.resolve(summary()) : Promise.resolve(),
    );

    render(<PortableSection reload={() => {}} />);
    fireEvent.click(screen.getByText("import JSON"));

    await waitFor(() => {
      expect(screen.getByText("Replace all current configuration?")).toBeTruthy();
    });
    expect(screen.getByText("Profiles: 2")).toBeTruthy();
    expect(screen.getByText("Active profile after import: imported")).toBeTruthy();
    expect(screen.getByText(/Light palette/)).toBeTruthy();
    expect(
      screen.getByText("Device selection resets to automatic for every profile."),
    ).toBeTruthy();
    expect(screen.getByText("keymap [imported]: embedded")).toBeTruthy();
    expect(screen.getByText("keymap [spare]: none")).toBeTruthy();
  });

  it("blocks an invalid import and surfaces the validation error", async () => {
    openMock.mockResolvedValue("/tmp/bad.json");
    invokeMock.mockImplementation((cmd: string) =>
      cmd === "inspect_import"
        ? Promise.reject(new Error("unsupported portable version 99"))
        : Promise.resolve(),
    );

    render(<PortableSection reload={() => {}} />);
    fireEvent.click(screen.getByText("import JSON"));

    await waitFor(() => {
      expect(
        screen.getByText(/unsupported portable version 99/),
      ).toBeTruthy();
    });
    expect(screen.queryByText("Replace all current configuration?")).toBeNull();
  });

  it("cancels a previewed import without making changes", async () => {
    openMock.mockResolvedValue("/tmp/backup.json");
    invokeMock.mockImplementation((cmd: string) =>
      cmd === "inspect_import" ? Promise.resolve(summary()) : Promise.resolve(),
    );

    render(<PortableSection reload={() => {}} />);
    fireEvent.click(screen.getByText("import JSON"));

    await waitFor(() => {
      expect(screen.getByText("Replace all current configuration?")).toBeTruthy();
    });

    fireEvent.click(screen.getByText("cancel"));

    await waitFor(() => {
      expect(screen.queryByText("Replace all current configuration?")).toBeNull();
    });
    const commitCalls = invokeMock.mock.calls.filter(([cmd]) => cmd === "commit_import");
    expect(commitCalls.length).toBe(0);
  });

  it("confirms replacement, commits, and reloads settings", async () => {
    const reload = vi.fn();
    openMock.mockResolvedValue("/tmp/backup.json");
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "inspect_import") return Promise.resolve(summary());
      if (cmd === "commit_import") return Promise.resolve();
      return Promise.resolve();
    });

    render(<PortableSection reload={reload} />);
    fireEvent.click(screen.getByText("import JSON"));

    await waitFor(() => {
      expect(screen.getByText("replace all configuration")).toBeTruthy();
    });

    fireEvent.click(screen.getByText("replace all configuration"));

    await waitFor(() => {
      const commitCalls = invokeMock.mock.calls.filter(
        ([cmd]) => cmd === "commit_import",
      );
      expect(commitCalls.length).toBe(1);
      expect(commitCalls[0][1]).toEqual({ path: "/tmp/backup.json" });
    });
    await waitFor(() => {
      expect(reload).toHaveBeenCalled();
    });
    expect(screen.getByText(/Configuration replaced/)).toBeTruthy();
  });

  it("reports export success with the chosen destination", async () => {
    saveMock.mockResolvedValue("/tmp/kb-hud-config.json");
    invokeMock.mockImplementation((cmd: string) =>
      cmd === "export_configuration" ? Promise.resolve() : Promise.resolve(),
    );

    render(<PortableSection reload={() => {}} />);
    fireEvent.click(screen.getByText("export JSON"));

    await waitFor(() => {
      expect(screen.getByText(/Exported to \/tmp\/kb-hud-config.json/)).toBeTruthy();
    });
  });

  it("reports export failure feedback", async () => {
    saveMock.mockResolvedValue("/tmp/kb-hud-config.json");
    invokeMock.mockImplementation((cmd: string) =>
      cmd === "export_configuration"
        ? Promise.reject(new Error("could not read keymap"))
        : Promise.resolve(),
    );

    render(<PortableSection reload={() => {}} />);
    fireEvent.click(screen.getByText("export JSON"));

    await waitFor(() => {
      expect(screen.getByText(/Export failed/)).toBeTruthy();
    });
  });
});

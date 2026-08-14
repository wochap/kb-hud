import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open as openFileDialog, save as saveFileDialog } from "@tauri-apps/plugin-dialog";

import { parseKeymapSvg } from "../keymap/parser";
import {
  DEFAULT_HUD_VISIBILITY,
  DEFAULT_OVERLAY_APPEARANCE,
  type Appearance,
  type HudVisibility,
  type OverlayAppearance,
  type Profile,
  type ThemeId,
} from "../profile";
import type { ImportSummary } from "../portable";
import { THEME_LABELS } from "../theme";

import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Separator } from "@/components/ui/separator";
import { Slider } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";

interface PairedDevice {
  name: string;
  address: string;
}

const TOP_BAR_TOGGLES: { key: keyof HudVisibility; label: string }[] = [
  { key: "layer", label: "Layer badge" },
  { key: "connection", label: "Connection status" },
  { key: "gaps", label: "Sequence gaps" },
  { key: "firmwareDrops", label: "Firmware drops" },
  { key: "battery", label: "Battery levels" },
  { key: "transport", label: "Transport (BLE/USB)" },
  { key: "modifiers", label: "Active modifiers" },
];

const THEME_OPTIONS: ThemeId[] = ["latte", "frappe", "macchiato", "mocha"];

type SvgFeedback =
  | { kind: "ok"; layers: number; positions: number }
  | { kind: "error"; message: string }
  | null;

function useProfiles() {
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [activeName, setActiveName] = useState("");

  const reload = useCallback(async () => {
    try {
      const [list, active] = await Promise.all([
        invoke<Profile[]>("list_profiles"),
        invoke<Profile>("get_active_profile"),
      ]);
      setProfiles(list);
      setActiveName(active.name);
    } catch {
      // backend not available (plain vite dev)
    }
  }, []);

  useEffect(() => {
    reload();
  }, [reload]);

  return { profiles, activeName, reload };
}

function useAppearance() {
  const [appearance, setAppearance] = useState<Appearance | null>(null);

  const reload = useCallback(async () => {
    try {
      setAppearance(await invoke<Appearance>("get_global_appearance"));
    } catch {
      // backend not available
    }
  }, []);

  useEffect(() => {
    reload();
  }, [reload]);

  const update = useCallback(
    async (patch: Partial<Appearance>) => {
      try {
        const updated = await invoke<Appearance>("update_global_appearance", {
          patch,
        });
        setAppearance(updated);
      } catch {
        // ignore
      }
    },
    [],
  );

  return { appearance, update, reload };
}

function SectionCard({
  title,
  description,
  children,
}: {
  title: string;
  description?: string;
  children: React.ReactNode;
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-base">{title}</CardTitle>
        {description && <CardDescription>{description}</CardDescription>}
      </CardHeader>
      <CardContent className="space-y-4">{children}</CardContent>
    </Card>
  );
}

function ProfilesSection({
  profiles,
  activeName,
  reload,
}: {
  profiles: Profile[];
  activeName: string;
  reload: () => void;
}) {
  const [newName, setNewName] = useState("");
  const [renaming, setRenaming] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const [error, setError] = useState<string | null>(null);

  const run = async (fn: () => Promise<unknown>) => {
    try {
      setError(null);
      await fn();
      await reload();
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <SectionCard title="Profiles">
      <div className="space-y-2">
        {profiles.map((profile) => (
          <div key={profile.name} className="flex items-center gap-2">
            <input
              type="radio"
              id={`profile-${profile.name}`}
              name="active-profile"
              className="accent-primary"
              checked={profile.name === activeName}
              onChange={() =>
                run(() => invoke("set_active_profile", { name: profile.name }))
              }
            />
            {renaming === profile.name ? (
              <div className="flex flex-1 items-center gap-2">
                <Input
                  value={renameValue}
                  onChange={(e) => setRenameValue(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter")
                      run(() =>
                        invoke("rename_profile", {
                          name: profile.name,
                          newName: renameValue,
                        }),
                      ).then(() => setRenaming(null));
                    if (e.key === "Escape") setRenaming(null);
                  }}
                />
                <Button
                  size="sm"
                  onClick={() =>
                    run(() =>
                      invoke("rename_profile", {
                        name: profile.name,
                        newName: renameValue,
                      }),
                    ).then(() => setRenaming(null))
                  }
                >
                  save
                </Button>
              </div>
            ) : (
              <label
                htmlFor={`profile-${profile.name}`}
                className={
                  profile.name === activeName
                    ? "flex-1 cursor-pointer font-medium"
                    : "flex-1 cursor-pointer"
                }
              >
                {profile.name}
              </label>
            )}
            <Button
              variant="ghost"
              size="sm"
              onClick={() => {
                setRenaming(profile.name);
                setRenameValue(profile.name);
              }}
            >
              rename
            </Button>
            <Button
              variant="ghost"
              size="sm"
              disabled={profile.name === activeName}
              onClick={() =>
                run(() => invoke("delete_profile", { name: profile.name }))
              }
            >
              delete
            </Button>
          </div>
        ))}
      </div>
      <Separator />
      <div className="flex items-center gap-2">
        <Input
          placeholder="new profile name"
          value={newName}
          onChange={(e) => setNewName(e.target.value)}
        />
        <Button
          onClick={() =>
            run(() => invoke("create_profile", { name: newName })).then(() =>
              setNewName(""),
            )
          }
        >
          create
        </Button>
      </div>
      {error && (
        <Alert variant="destructive">
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}
    </SectionCard>
  );
}

export function AppearanceSection({
  appearance,
  update,
}: {
  appearance: Appearance | null;
  update: (patch: Partial<Appearance>) => void;
}) {
  return (
    <SectionCard
      title="Appearance"
      description="Palette applied automatically by the system light/dark mode."
    >
      <div className="grid grid-cols-2 gap-4">
        <div className="space-y-2">
          <Label htmlFor="light-theme">Light mode palette</Label>
          <Select
            value={appearance?.lightTheme}
            onValueChange={(value) => update({ lightTheme: value as ThemeId })}
          >
            <SelectTrigger id="light-theme">
              <SelectValue placeholder="Select palette" />
            </SelectTrigger>
            <SelectContent>
              {THEME_OPTIONS.map((id) => (
                <SelectItem key={id} value={id}>
                  {THEME_LABELS[id]}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        <div className="space-y-2">
          <Label htmlFor="dark-theme">Dark mode palette</Label>
          <Select
            value={appearance?.darkTheme}
            onValueChange={(value) => update({ darkTheme: value as ThemeId })}
          >
            <SelectTrigger id="dark-theme">
              <SelectValue placeholder="Select palette" />
            </SelectTrigger>
            <SelectContent>
              {THEME_OPTIONS.map((id) => (
                <SelectItem key={id} value={id}>
                  {THEME_LABELS[id]}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </div>
    </SectionCard>
  );
}

function KeymapSection({
  profile,
  reload,
}: {
  profile: Profile;
  reload: () => void;
}) {
  const [path, setPath] = useState(profile.svgPath);
  const [feedback, setFeedback] = useState<SvgFeedback>(null);

  useEffect(() => {
    setPath(profile.svgPath);
    setFeedback(null);
  }, [profile.name, profile.svgPath]);

  const apply = async () => {
    try {
      const svgText = await invoke<string>("read_keymap_svg", { path });
      const keymap = parseKeymapSvg(svgText);
      await invoke("update_profile", {
        name: profile.name,
        patch: { svgPath: path },
      });
      setFeedback({
        kind: "ok",
        layers: keymap.layers.length,
        positions: keymap.positions.length,
      });
      reload();
    } catch (e) {
      setFeedback({ kind: "error", message: String(e) });
    }
  };

  return (
    <SectionCard title="Keymap SVG">
      <div className="flex items-center gap-2">
        <Input
          value={path}
          placeholder="/path/to/keymap.svg"
          onChange={(e) => setPath(e.target.value)}
        />
        <Button onClick={apply}>apply</Button>
      </div>
      {feedback?.kind === "ok" && (
        <p className="text-sm text-muted-foreground">
          parsed: {feedback.layers} layers, {feedback.positions} positions
        </p>
      )}
      {feedback?.kind === "error" && (
        <Alert variant="destructive">
          <AlertDescription>{feedback.message}</AlertDescription>
        </Alert>
      )}
    </SectionCard>
  );
}

export function DeviceSection({
  profile,
  reload,
}: {
  profile: Profile;
  reload: () => void;
}) {
  const isAuto = profile.deviceMac === "auto";
  const [mac, setMac] = useState(isAuto ? "" : profile.deviceMac);
  const [devices, setDevices] = useState<PairedDevice[] | null>(null);
  const [deviceError, setDeviceError] = useState<string | null>(null);

  useEffect(() => {
    setMac(profile.deviceMac === "auto" ? "" : profile.deviceMac);
  }, [profile.name, profile.deviceMac]);

  const setDevice = async (deviceMac: string) => {
    await invoke("update_profile", {
      name: profile.name,
      patch: { deviceMac },
    });
    reload();
  };

  const refreshDevices = async () => {
    setDeviceError(null);
    try {
      setDevices(await invoke<PairedDevice[]>("list_bluetooth_devices"));
    } catch (e) {
      setDevices(null);
      setDeviceError(String(e));
    }
  };

  return (
    <SectionCard title="Device">
      <div className="space-y-2">
        <div className="flex items-center gap-2">
          <input
            type="radio"
            id="device-auto"
            name="device-mode"
            className="accent-primary"
            checked={isAuto}
            onChange={() => setDevice("auto")}
          />
          <Label htmlFor="device-auto" className="cursor-pointer font-normal">
            auto (compatible zmk-key-telemetry keyboard)
          </Label>
        </div>
        <div className="flex items-center gap-2">
          <input
            type="radio"
            id="device-explicit"
            name="device-mode"
            className="accent-primary"
            checked={!isAuto}
            onChange={() => mac && setDevice(mac)}
          />
          <Label
            htmlFor="device-explicit"
            className="cursor-pointer font-normal"
          >
            explicit MAC
          </Label>
          <Input
            placeholder="AA:BB:CC:DD:EE:FF"
            value={mac}
            onChange={(e) => setMac(e.target.value)}
            onBlur={() => !isAuto && mac && setDevice(mac)}
            className="max-w-xs"
          />
        </div>
      </div>
      <div className="flex items-center gap-2">
        <Button variant="outline" size="sm" onClick={refreshDevices}>
          list paired devices
        </Button>
        <Button
          variant="outline"
          size="sm"
          onClick={() => invoke("ble_reconnect").catch(() => {})}
        >
          reconnect now
        </Button>
      </div>
      {deviceError && (
        <Alert variant="destructive">
          <AlertDescription>{deviceError}</AlertDescription>
        </Alert>
      )}
      {devices && (
        <ul className="space-y-1 text-sm">
          {devices.length === 0 && (
            <li className="text-muted-foreground">no paired devices</li>
          )}
          {devices.map((device) => (
            <li key={device.address} className="flex items-center gap-2">
              <span className="flex-1">
                {device.name} — {device.address}
              </span>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setDevice(device.address)}
              >
                use
              </Button>
            </li>
          ))}
        </ul>
      )}
    </SectionCard>
  );
}

function ScaleSection({
  profile,
  reload,
}: {
  profile: Profile;
  reload: () => void;
}) {
  const [preview, setPreview] = useState(profile.scale);

  useEffect(() => {
    setPreview(profile.scale);
  }, [profile.name, profile.scale]);

  return (
    <SectionCard title="Scale">
      <div className="flex items-center gap-4">
        <Slider
          min={0.4}
          max={2}
          step={0.05}
          value={[preview]}
          onValueChange={([value]) => setPreview(value)}
          onValueCommit={([value]) =>
            invoke("update_profile", {
              name: profile.name,
              patch: { scale: value },
            }).then(reload)
          }
          className="flex-1"
        />
        <span className="w-14 text-right text-sm tabular-nums">
          {preview.toFixed(2)}×
        </span>
      </div>
    </SectionCard>
  );
}

function TopBarSection({
  profile,
  reload,
}: {
  profile: Profile;
  reload: () => void;
}) {
  const hud = profile.hud ?? DEFAULT_HUD_VISIBILITY;

  const toggle = async (key: keyof HudVisibility) => {
    await invoke("update_profile", {
      name: profile.name,
      patch: { hud: { ...hud, [key]: !hud[key] } },
    });
    reload();
  };

  return (
    <SectionCard title="Top bar">
      <div className="grid grid-cols-2 gap-3">
        {TOP_BAR_TOGGLES.map(({ key, label }) => (
          <div key={key} className="flex items-center justify-between gap-2">
            <Label htmlFor={`hud-${key}`} className="cursor-pointer font-normal">
              {label}
            </Label>
            <Switch
              id={`hud-${key}`}
              checked={hud[key]}
              onCheckedChange={() => toggle(key)}
            />
          </div>
        ))}
      </div>
    </SectionCard>
  );
}

interface OpacitySliderProps {
  label: string;
  value: number;
  onPreview: (value: number) => void;
  onCommit: (value: number) => void;
}

function OpacitySlider({ label, value, onPreview, onCommit }: OpacitySliderProps) {
  return (
    <div className="space-y-1.5">
      <div className="flex items-center justify-between">
        <Label className="font-normal">{label}</Label>
        <span className="text-sm tabular-nums text-muted-foreground">
          {`${Math.round(value * 100)}%`}
        </span>
      </div>
      <Slider
        min={0}
        max={1}
        step={0.01}
        value={[value]}
        onValueChange={([v]) => onPreview(v)}
        onValueCommit={([v]) => onCommit(v)}
      />
    </div>
  );
}

export function OverlayAppearanceSection({
  profile,
  reload,
}: {
  profile: Profile;
  reload: () => void;
}) {
  const [overlay, setOverlay] = useState<OverlayAppearance>(
    profile.overlayAppearance ?? DEFAULT_OVERLAY_APPEARANCE,
  );

  useEffect(() => {
    setOverlay(profile.overlayAppearance ?? DEFAULT_OVERLAY_APPEARANCE);
  }, [profile.name, profile.overlayAppearance]);

  const persist = async (next: OverlayAppearance) => {
    await invoke("update_profile", {
      name: profile.name,
      patch: { overlayAppearance: next },
    });
    reload();
  };

  const preview = (patch: Partial<OverlayAppearance>) =>
    setOverlay((current) => ({ ...current, ...patch }));

  const commit = (patch: Partial<OverlayAppearance>) =>
    persist({ ...overlay, ...patch });

  return (
    <SectionCard
      title="Overlay appearance"
      description="Visibility and opacity for this profile's keyboard overlay."
    >
      <div className="flex items-center justify-between gap-2">
        <Label htmlFor="idle-backgrounds" className="cursor-pointer font-normal">
          Show idle key backgrounds
        </Label>
        <Switch
          id="idle-backgrounds"
          checked={overlay.showIdleKeyBackgrounds}
          onCheckedChange={(checked) => {
            const next = { ...overlay, showIdleKeyBackgrounds: checked };
            setOverlay(next);
            persist(next);
          }}
        />
      </div>
      <Separator />
      <div className="space-y-4">
        <OpacitySlider
          label="Label opacity"
          value={overlay.labelOpacity}
          onPreview={(v) => preview({ labelOpacity: v })}
          onCommit={(v) => commit({ labelOpacity: v })}
        />
        <OpacitySlider
          label="Idle key background opacity"
          value={overlay.idleKeyBackgroundOpacity}
          onPreview={(v) => preview({ idleKeyBackgroundOpacity: v })}
          onCommit={(v) => commit({ idleKeyBackgroundOpacity: v })}
        />
        <OpacitySlider
          label="Key border opacity"
          value={overlay.keyBorderOpacity}
          onPreview={(v) => preview({ keyBorderOpacity: v })}
          onCommit={(v) => commit({ keyBorderOpacity: v })}
        />
        <OpacitySlider
          label="Active key background opacity"
          value={overlay.activeKeyBackgroundOpacity}
          onPreview={(v) => preview({ activeKeyBackgroundOpacity: v })}
          onCommit={(v) => commit({ activeKeyBackgroundOpacity: v })}
        />
        <OpacitySlider
          label="Top-bar pill background opacity"
          value={overlay.topBarPillBackgroundOpacity}
          onPreview={(v) => preview({ topBarPillBackgroundOpacity: v })}
          onCommit={(v) => commit({ topBarPillBackgroundOpacity: v })}
        />
      </div>
    </SectionCard>
  );
}

type PortableFeedback =
  | { kind: "success"; message: string }
  | { kind: "error"; message: string }
  | null;

export function PortableSection({ reload }: { reload: () => void }) {
  const [summary, setSummary] = useState<ImportSummary | null>(null);
  const [importPath, setImportPath] = useState<string | null>(null);
  const [feedback, setFeedback] = useState<PortableFeedback>(null);
  const [busy, setBusy] = useState(false);

  const exportConfig = async () => {
    setFeedback(null);
    try {
      const path = await saveFileDialog({
        title: "Export kb-hud configuration",
        defaultPath: "kb-hud-config.json",
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (!path) return;
      await invoke("export_configuration", { path });
      setFeedback({ kind: "success", message: `Exported to ${path}` });
    } catch (e) {
      setFeedback({ kind: "error", message: `Export failed: ${e}` });
    }
  };

  const chooseImport = async () => {
    setFeedback(null);
    setSummary(null);
    setImportPath(null);
    try {
      const path = await openFileDialog({
        title: "Import kb-hud configuration",
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (!path || typeof path !== "string") return;
      const preview = await invoke<ImportSummary>("inspect_import", { path });
      setImportPath(path);
      setSummary(preview);
    } catch (e) {
      setFeedback({ kind: "error", message: `Import validation failed: ${e}` });
    }
  };

  const confirmImport = async () => {
    if (!importPath) return;
    setBusy(true);
    setFeedback(null);
    try {
      await invoke("commit_import", { path: importPath });
      setSummary(null);
      setImportPath(null);
      setFeedback({
        kind: "success",
        message: "Configuration replaced. Settings and overlay reloaded.",
      });
      reload();
    } catch (e) {
      setFeedback({ kind: "error", message: `Import failed: ${e}` });
    } finally {
      setBusy(false);
    }
  };

  const cancelImport = () => {
    setSummary(null);
    setImportPath(null);
    setFeedback(null);
  };

  return (
    <SectionCard
      title="Portable configuration"
      description="Export a backup or replace this machine's configuration from an export. Bluetooth addresses and keymap paths are never included."
    >
      <div className="flex items-center gap-2">
        <Button variant="outline" onClick={exportConfig}>
          export JSON
        </Button>
        <Button variant="outline" onClick={chooseImport}>
          import JSON
        </Button>
      </div>

      {feedback?.kind === "success" && (
        <Alert>
          <AlertDescription>{feedback.message}</AlertDescription>
        </Alert>
      )}
      {feedback?.kind === "error" && (
        <Alert variant="destructive">
          <AlertDescription>{feedback.message}</AlertDescription>
        </Alert>
      )}

      {summary && (
        <div className="space-y-3 rounded-lg border border-destructive/50 p-4">
          <p className="text-sm font-medium text-destructive">
            Replace all current configuration?
          </p>
          <ul className="space-y-1 text-sm">
            <li>Profiles: {summary.profileCount}</li>
            <li>Active profile after import: {summary.activeProfile}</li>
            <li>
              Light palette: {THEME_LABELS[summary.lightTheme as ThemeId] ?? summary.lightTheme}
            </li>
            <li>
              Dark palette: {THEME_LABELS[summary.darkTheme as ThemeId] ?? summary.darkTheme}
            </li>
            <li>Device selection resets to automatic for every profile.</li>
          </ul>
          <ul className="space-y-1 text-sm">
            {summary.keymaps.map((keymap) => (
              <li key={keymap.profile} className="text-muted-foreground">
                keymap [{keymap.profile}]: {keymap.status}
              </li>
            ))}
          </ul>
          <p className="text-sm text-muted-foreground">
            This cannot be undone. Your current profiles and appearance settings
            will be overwritten.
          </p>
          <div className="flex items-center gap-2">
            <Button
              variant="destructive"
              disabled={busy}
              onClick={confirmImport}
            >
              {busy ? "importing…" : "replace all configuration"}
            </Button>
            <Button variant="ghost" disabled={busy} onClick={cancelImport}>
              cancel
            </Button>
          </div>
        </div>
      )}
    </SectionCard>
  );
}

function DevPanel() {
  const [position, setPosition] = useState(15);
  const [burst, setBurst] = useState(8);
  const [layer, setLayer] = useState(3);
  const [modifier, setModifier] = useState(1);

  const cmd = (name: string, args?: Record<string, unknown>) =>
    invoke(name, args).catch(() => {});

  return (
    <SectionCard
      title="Mock telemetry dev panel"
      description="Inject synthetic telemetry events for development."
    >
      <div className="grid grid-cols-2 gap-3">
        <div className="flex items-center gap-2">
          <Label className="font-normal">position</Label>
          <Input
            type="number"
            min={0}
            max={63}
            value={position}
            onChange={(e) => setPosition(Number(e.target.value))}
            className="w-20"
          />
          <Button size="sm" variant="outline" onClick={() => cmd("mock_press", { position })}>
            press
          </Button>
          <Button size="sm" variant="outline" onClick={() => cmd("mock_release", { position })}>
            release
          </Button>
        </div>
        <div className="flex items-center gap-2">
          <Label className="font-normal">burst</Label>
          <Input
            type="number"
            min={1}
            max={64}
            value={burst}
            onChange={(e) => setBurst(Number(e.target.value))}
            className="w-20"
          />
          <Button size="sm" variant="outline" onClick={() => cmd("mock_burst", { count: burst })}>
            random burst
          </Button>
        </div>
        <div className="flex items-center gap-2">
          <Label className="font-normal">layer</Label>
          <Input
            type="number"
            min={0}
            max={31}
            value={layer}
            onChange={(e) => setLayer(Number(e.target.value))}
            className="w-20"
          />
          <Button size="sm" variant="outline" onClick={() => cmd("mock_hold_layer", { layer })}>
            hold
          </Button>
          <Button size="sm" variant="outline" onClick={() => cmd("mock_release_layer", { layer })}>
            release
          </Button>
        </div>
        <div className="flex items-center gap-2">
          <Label className="font-normal">modifier</Label>
          <select
            value={modifier}
            onChange={(e) => setModifier(Number(e.target.value))}
            className="h-9 rounded-md border border-input bg-transparent px-2 text-sm"
          >
            {["LCTRL", "LSHIFT", "LALT", "LGUI", "RCTRL", "RSHIFT", "RALT", "RGUI"].map(
              (name, bit) => (
                <option value={bit} key={name}>
                  {name}
                </option>
              ),
            )}
          </select>
          <Button
            size="sm"
            variant="outline"
            onClick={() => cmd("mock_set_modifier", { bit: modifier, active: true })}
          >
            down
          </Button>
          <Button
            size="sm"
            variant="outline"
            onClick={() => cmd("mock_set_modifier", { bit: modifier, active: false })}
          >
            up
          </Button>
        </div>
      </div>
      <Separator />
      <div className="flex flex-wrap items-center gap-2">
        <Button size="sm" variant="outline" onClick={() => cmd("mock_set_demo_status", { enabled: true })}>
          demo status on
        </Button>
        <Button size="sm" variant="outline" onClick={() => cmd("mock_set_demo_status", { enabled: false })}>
          demo status off
        </Button>
        <Button size="sm" variant="outline" onClick={() => cmd("mock_inject_firmware_drop")}>
          inject fw drop
        </Button>
        <Button size="sm" variant="outline" onClick={() => cmd("mock_inject_gap")}>
          inject gap
        </Button>
        <Button size="sm" variant="outline" onClick={() => cmd("mock_disconnect")}>
          disconnect
        </Button>
        <Button size="sm" variant="outline" onClick={() => cmd("mock_reconnect")}>
          reconnect
        </Button>
      </div>
    </SectionCard>
  );
}

export function SettingsView() {
  const { profiles, activeName, reload } = useProfiles();
  const { appearance, update } = useAppearance();
  const active = profiles.find((p) => p.name === activeName) ?? null;

  return (
    <main className="mx-auto max-w-3xl space-y-4 p-6">
      <h1 className="text-xl font-semibold">kb-hud settings</h1>
      <ProfilesSection profiles={profiles} activeName={activeName} reload={reload} />
      <AppearanceSection appearance={appearance} update={update} />
      {active && (
        <>
          <KeymapSection profile={active} reload={reload} />
          <DeviceSection profile={active} reload={reload} />
          <ScaleSection profile={active} reload={reload} />
          <TopBarSection profile={active} reload={reload} />
          <OverlayAppearanceSection profile={active} reload={reload} />
        </>
      )}
      <PortableSection reload={reload} />
      <DevPanel />
    </main>
  );
}

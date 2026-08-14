import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

import { parseKeymapSvg } from "../keymap/parser";
import {
  DEFAULT_HUD_VISIBILITY,
  type HudVisibility,
  type Profile,
} from "../profile";
import "./settings.css";

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
    <section>
      <h2>Profiles</h2>
      <ul className="profile-list">
        {profiles.map((profile) => (
          <li key={profile.name} className="profile-row">
            <label>
              <input
                type="radio"
                name="active-profile"
                checked={profile.name === activeName}
                onChange={() =>
                  run(() =>
                    invoke("set_active_profile", { name: profile.name }),
                  )
                }
              />
              {renaming === profile.name ? (
                <>
                  <input
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
                  <button
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
                  </button>
                </>
              ) : (
                <span
                  className={
                    profile.name === activeName ? "active-name" : undefined
                  }
                >
                  {profile.name}
                </span>
              )}
            </label>
            <span className="row-actions">
              <button
                onClick={() => {
                  setRenaming(profile.name);
                  setRenameValue(profile.name);
                }}
              >
                rename
              </button>
              <button
                disabled={profile.name === activeName}
                onClick={() =>
                  run(() => invoke("delete_profile", { name: profile.name }))
                }
              >
                delete
              </button>
            </span>
          </li>
        ))}
      </ul>
      <div className="row">
        <input
          placeholder="new profile name"
          value={newName}
          onChange={(e) => setNewName(e.target.value)}
        />
        <button
          onClick={() =>
            run(() => invoke("create_profile", { name: newName })).then(() =>
              setNewName(""),
            )
          }
        >
          create
        </button>
      </div>
      {error && <p className="error">{error}</p>}
    </section>
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
    <section>
      <h2>Keymap SVG</h2>
      <div className="row">
        <input
          value={path}
          placeholder="/path/to/keymap.svg"
          onChange={(e) => setPath(e.target.value)}
        />
        <button onClick={apply}>apply</button>
      </div>
      {feedback?.kind === "ok" && (
        <p className="success">
          parsed: {feedback.layers} layers, {feedback.positions} positions
        </p>
      )}
      {feedback?.kind === "error" && (
        <p className="error">{feedback.message}</p>
      )}
    </section>
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
    <section>
      <h2>Device</h2>
      <div className="row">
        <label>
          <input
            type="radio"
            checked={isAuto}
            onChange={() => setDevice("auto")}
          />
          auto (compatible zmk-key-telemetry keyboard)
        </label>
        <label>
          <input
            type="radio"
            checked={!isAuto}
            onChange={() => mac && setDevice(mac)}
          />
          explicit MAC
        </label>
        <input
          placeholder="AA:BB:CC:DD:EE:FF"
          value={mac}
          onChange={(e) => setMac(e.target.value)}
          onBlur={() => !isAuto && mac && setDevice(mac)}
        />
      </div>
      <div className="row">
        <button onClick={refreshDevices}>list paired devices</button>
        <button onClick={() => invoke("ble_reconnect").catch(() => {})}>
          reconnect now
        </button>
      </div>
      {deviceError && <p className="error">{deviceError}</p>}
      {devices && (
        <ul className="device-list">
          {devices.length === 0 && <li>no paired devices</li>}
          {devices.map((device) => (
            <li key={device.address}>
              {device.name} — {device.address}{" "}
              <button onClick={() => setDevice(device.address)}>use</button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

function ScaleSection({
  profile,
  reload,
}: {
  profile: Profile;
  reload: () => void;
}) {
  return (
    <section>
      <h2>Scale</h2>
      <div className="row">
        <input
          type="range"
          min={0.4}
          max={2}
          step={0.05}
          value={profile.scale}
          onChange={(e) =>
            invoke("update_profile", {
              name: profile.name,
              patch: { scale: Number(e.target.value) },
            }).then(reload)
          }
        />
        <span className="scale-value">{profile.scale.toFixed(2)}×</span>
      </div>
    </section>
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
    <section>
      <h2>Top bar</h2>
      <ul className="topbar-list">
        {TOP_BAR_TOGGLES.map(({ key, label }) => (
          <li key={key}>
            <label>
              <input
                type="checkbox"
                checked={hud[key]}
                onChange={() => toggle(key)}
              />
              {label}
            </label>
          </li>
        ))}
      </ul>
    </section>
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
    <section>
      <h2>Mock telemetry dev panel</h2>
      <div className="row">
        <label>
          position
          <input
            type="number"
            min={0}
            max={63}
            value={position}
            onChange={(e) => setPosition(Number(e.target.value))}
          />
        </label>
        <button onClick={() => cmd("mock_press", { position })}>press</button>
        <button onClick={() => cmd("mock_release", { position })}>
          release
        </button>
      </div>
      <div className="row">
        <label>
          burst
          <input
            type="number"
            min={1}
            max={64}
            value={burst}
            onChange={(e) => setBurst(Number(e.target.value))}
          />
        </label>
        <button onClick={() => cmd("mock_burst", { count: burst })}>
          random burst
        </button>
      </div>
      <div className="row">
        <label>
          layer
          <input
            type="number"
            min={0}
            max={31}
            value={layer}
            onChange={(e) => setLayer(Number(e.target.value))}
          />
        </label>
        <button onClick={() => cmd("mock_hold_layer", { layer })}>
          hold layer
        </button>
        <button onClick={() => cmd("mock_release_layer", { layer })}>
          release layer
        </button>
      </div>
      <div className="row">
        <label>
          modifier
          <select
            value={modifier}
            onChange={(e) => setModifier(Number(e.target.value))}
          >
            {[
              "LCTRL",
              "LSHIFT",
              "LALT",
              "LGUI",
              "RCTRL",
              "RSHIFT",
              "RALT",
              "RGUI",
            ].map((name, bit) => (
              <option value={bit} key={name}>{name}</option>
            ))}
          </select>
        </label>
        <button
          onClick={() =>
            cmd("mock_set_modifier", { bit: modifier, active: true })
          }
        >
          modifier down
        </button>
        <button
          onClick={() =>
            cmd("mock_set_modifier", { bit: modifier, active: false })
          }
        >
          modifier up
        </button>
      </div>
      <div className="row">
        <button onClick={() => cmd("mock_set_demo_status", { enabled: true })}>
          demo status on
        </button>
        <button onClick={() => cmd("mock_set_demo_status", { enabled: false })}>
          demo status off
        </button>
        <button onClick={() => cmd("mock_inject_firmware_drop")}>
          inject fw drop
        </button>
      </div>
      <div className="row">
        <button onClick={() => cmd("mock_inject_gap")}>inject gap</button>
        <button onClick={() => cmd("mock_disconnect")}>disconnect</button>
        <button onClick={() => cmd("mock_reconnect")}>reconnect</button>
      </div>
    </section>
  );
}

export function SettingsView() {
  const { profiles, activeName, reload } = useProfiles();
  const active = profiles.find((p) => p.name === activeName) ?? null;

  return (
    <main className="settings">
      <h1>kb-hud settings</h1>
      <ProfilesSection
        profiles={profiles}
        activeName={activeName}
        reload={reload}
      />
      {active && (
        <>
          <KeymapSection profile={active} reload={reload} />
          <DeviceSection profile={active} reload={reload} />
          <ScaleSection profile={active} reload={reload} />
          <TopBarSection profile={active} reload={reload} />
        </>
      )}
      <DevPanel />
    </main>
  );
}

// Typed wrappers for the Rust commands in src-tauri/src/lib.rs.
import { invoke } from "@tauri-apps/api/core";

export type Zone = "main" | "side";
export type RGB = [number, number, number];

export interface Lighting {
  mode: number;
  speed: number; // 0 slowest .. 4 fastest
  brightness: number; // 0 .. 4
  option: number; // mode-specific variant, index into ModeInfo.options
  color: number; // 0..6 preset, 7 custom rgb, 8 rainbow
  rgb: RGB;
}

export interface ModeInfo {
  id: number;
  name: string;
  speed: boolean;
  color: boolean; // whether the effect uses the color setting
  options: string[];
}

// A key in the Striker's drawing: which per-key slot it uses and where it is drawn.
export interface Key {
  label: string;
  slot: number;
  x: number;
  y: number;
  w: number;
  h: number;
}

export type Picture = RGB[]; // one color per key slot, 128 slots

export interface Info {
  firmware: number; // 0x0105 = 1.05
  device_id: number;
  wireless: boolean;
}

export interface SleepTimers {
  // seconds; only used on Bluetooth and 2.4 GHz
  light_bt: number;
  light_24g: number;
  deep_bt: number;
  deep_24g: number;
}

export interface Settings {
  polling_hz: number;
  debounce_ms: number;
  win_lock: boolean;
  mac_mode: boolean; // Fn shortcuts for macOS instead of Windows
  sleep: SleepTimers;
}

export type Frame = boolean[][]; // frame[y][x], 9x9

// Stored with each screen: an animation keeps its frame strip and speed even at one frame.
export type Kind = "image" | "animation";

export interface Animation {
  slug: string;
  name: string;
  about: string;
  kind: Kind;
  delay: number; // ~20 ms units; the keyboard plays every frame at this one speed
  frames: Frame[];
}

export const MAX_FRAMES = 50;

export const COLOR_CUSTOM = 7;
export const COLOR_RAINBOW = 8;
export const MODE_PICTURE = 13; // per-key colors; the option picks picture 1..3
export const DELAY_UNIT_MS = 20;

// Swatches are sent as custom RGB (color 7), the path the official app uses.
export const SWATCHES: { name: string; rgb: RGB }[] = [
  { name: "Red", rgb: [255, 0, 0] },
  { name: "Orange", rgb: [255, 128, 0] },
  { name: "Yellow", rgb: [255, 255, 0] },
  { name: "Green", rgb: [0, 255, 0] },
  { name: "Cyan", rgb: [0, 255, 255] },
  { name: "Blue", rgb: [0, 0, 255] },
  { name: "Purple", rgb: [255, 0, 255] },
  { name: "White", rgb: [255, 255, 255] },
];

export const api = {
  info: () => invoke<Info>("info"),
  modes: (zone: Zone) => invoke<ModeInfo[]>("modes", { zone }),
  getLighting: (zone: Zone) => invoke<Lighting>("get_lighting", { zone }),
  setLighting: (zone: Zone, lighting: Lighting) => invoke<void>("set_lighting", { zone, lighting }),
  keys: () => invoke<Key[]>("keys"),
  getPicture: (picture: number) => invoke<Picture>("get_picture", { picture }),
  setPicture: (picture: number, colors: Picture) => invoke<void>("set_picture", { picture, colors }),
  library: () => invoke<Animation[]>("library"),
  saveScreen: (slug: string | null, name: string, kind: Kind, delay: number, frames: Frame[]) =>
    invoke<Animation>("save_screen", { slug, name, kind, delay, frames }),
  deleteScreen: (slug: string) => invoke<void>("delete_screen", { slug }),
  restoreIncluded: () => invoke<void>("restore_included"),
  screensFolder: () => invoke<string>("screens_folder"),
  play: (frames: Frame[], delay: number) => invoke<void>("play", { frames, delay }),
  getSettings: () => invoke<Settings>("get_settings"),
  setPollingRate: (hz: number) => invoke<void>("set_polling_rate", { hz }),
  setDebounce: (ms: number) => invoke<void>("set_debounce", { ms }),
  setWinLock: (locked: boolean) => invoke<void>("set_win_lock", { locked }),
  setMacMode: (mac: boolean) => invoke<void>("set_mac_mode", { mac }),
  setSleepTimers: (timers: SleepTimers) => invoke<void>("set_sleep_timers", { timers }),
};

export const blankFrame = (): Frame => Array.from({ length: 9 }, () => Array(9).fill(false));

export const toHex = ([r, g, b]: RGB) => "#" + [r, g, b].map((v) => v.toString(16).padStart(2, "0")).join("");
export const fromHex = (hex: string): RGB => [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16)) as RGB;

export const copyFrame = (f: Frame): Frame => f.map((row) => [...row]);

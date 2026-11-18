<p align="center">
  <img src="docs/hero.svg" alt="NetPulse — live download and upload speed in the Windows taskbar" width="100%">
</p>

<h1 align="center">NetPulse</h1>

<p align="center">
  <b>Live download and upload speed in your Windows 10 / 11 taskbar.</b><br>
  Native Win32 and Rust — no Electron, no background service, no traffic of its own.
</p>

<p align="center">
  <a href="https://github.com/Kaziopu5938/netpulse"><img alt="Platform" src="https://img.shields.io/badge/Windows-10%20%7C%2011-0EA5E9?style=flat-square&logo=windows&logoColor=white"></a>
  <img alt="Rust" src="https://img.shields.io/badge/Rust-stable-DEA584?style=flat-square&logo=rust&logoColor=white">
  <img alt="Memory" src="https://img.shields.io/badge/RAM-~11%20MB-34D399?style=flat-square">
  <img alt="Size" src="https://img.shields.io/badge/binary-4.9%20MiB-64748B?style=flat-square">
  <a href="LICENSE"><img alt="License" src="https://img.shields.io/badge/license-MIT-blue?style=flat-square"></a>
</p>

<p align="center">
  <img src="docs/images/taskbar.png" alt="NetPulse showing download and upload speed in the Windows 11 taskbar" width="860">
</p>

NetPulse reads the counters Windows already keeps for every network interface (`GetIfTable2`), turns
the difference between two readings into bytes per second, and draws the result in the taskbar band
next to the system tray. One second by default, no speed tests, no traffic, no GPU context in the
background process.

---

## Features

| | |
| --- | --- |
| **Live readout** | `↓ 12.5 Mbps ↑ 2.4 Mbps` — refresh every 250 ms to 5 s, 1 s by default |
| **Ethernet & Wi-Fi** | PPP and mobile broadband too; only physical interfaces are counted |
| **Adapter selection** | Automatic active-adapter detection, or pin one interface in Settings |
| **Honest idle/disconnect** | `0` when idle, `No network` when the link drops, counters that reset never go negative |
| **Compact & themeable** | Download only, upload only or both; Kbps/Mbps or KB/s/MB/s; font, size, colour, rounded background with real per-pixel opacity, tray side and gap |
| **Tray icon** | Tooltip with live speeds, Open Settings, Show/Hide, Network Information, Start with Windows, Check for Updates, Exit |
| **Settings window** | egui/eframe, system/light/dark theme, everything applied live and saved as JSON |
| **Multi-DPI & auto-hide aware** | Follows taskbar edge, thickness, DPI and auto-hide on a 1 s tick; rotated text on vertical taskbars |
| **Small** | ~11 MB working set, one 1 Hz sampler thread, 4.9 MiB self-contained executable |

## Taskbar integration — the honest version

This was the deciding design question, and it deserves a straight answer:

- **Windows 10** taskbars used to accept third-party *Desk Bands* / toolbars through COM shell
  extensions. They are deprecated, require code injected into `explorer.exe`, and break easily.
- **Windows 11 removed that surface completely.** There is no supported API that lets a third-party
  process add a text widget to the taskbar or put one *inside* the notification area.
- The only genuinely supported taskbar API is `Shell_NotifyIcon` — the tray icon. NetPulse uses it
  (via `tray-icon`) and shows live speeds in its tooltip. A tray icon cannot render arbitrary text
  next to itself.

NetPulse therefore implements two clearly separated display modes:

| Mode | What it actually is | Supported | Default |
| --- | --- | --- | --- |
| **Overlay** | A borderless, click-through (`WS_EX_TRANSPARENT`), no-activate, topmost layered window placed in the taskbar band beside `TrayNotifyWnd`. Text is drawn as an antialiased GDI mask, composited into a 32-bit premultiplied DIB and pushed with `UpdateLayeredWindow`, so it sits on any taskbar colour without fringing. It never takes clicks from taskbar buttons. | Yes | Yes |
| **Taskbar dock** | The same window converted to a `WS_CHILD` of Explorer's `Shell_TrayWnd` (`SetParent`), so Windows itself clips and moves it with the taskbar. Recreated after Explorer restarts (`TaskbarCreated`). | **Unsupported by Microsoft**; may break after Windows updates. | No |

Neither mode injects code into Explorer, and neither pretends to be a system taskbar control.

## Screenshots

<table>
  <tr>
    <td width="50%"><img src="docs/images/settings.png" alt="NetPulse settings window"><br><sub><b>Settings</b> — egui window, applied live</sub></td>
    <td width="50%"><img src="docs/images/tray-menu.png" alt="NetPulse tray menu"><br><sub><b>Tray menu</b> — operations without opening Settings</sub></td>
  </tr>
  <tr>
    <td colspan="2" align="center"><img src="docs/images/network-info.png" alt="NetPulse network information dialog" width="520"><br><sub><b>Network Information</b> — current adapter, sample speed and detected interfaces</sub></td>
  </tr>
</table>

## Install

### Build from source (recommended today)

Requirements: Windows 10/11 x64, [Rust stable](https://rustup.rs) and either the MSVC toolchain
(Visual Studio Build Tools) or a MinGW-w64 toolchain for the `x86_64-pc-windows-gnu` target.

```powershell
git clone https://github.com/Kaziopu5938/netpulse.git
cd netpulse
cargo build --release
.\target\release\netpulse.exe
```

`target\release\netpulse.exe` is standalone (no runtime DLLs with the MSVC or self-contained GNU
linker). Copy it anywhere and run it.

### Installer

`installer/netpulse.iss` is an [Inno Setup 6](https://jrsoftware.org/isinfo.php) script:

```powershell
cargo build --release
iscc installer\netpulse.iss
# installer\NetPulse-1.0.0-setup.exe
```

It installs per user (no admin prompt), optionally adds the autostart entry via
`netpulse.exe --enable-startup`, and removes the Run value plus `%APPDATA%\NetPulse` on uninstall.

### Autostart

Enable *Start with Windows* in Settings or from the tray menu. Under the hood it writes one value:

```
HKCU\Software\Microsoft\Windows\CurrentVersion\Run\NetPulse = "C:\path\to\netpulse.exe"
```

No scheduled task, no service, no admin rights.

## Usage

The tray menu:

- **Open Settings** — launches `netpulse.exe --settings` (a second, short-lived process).
- **Show Speed Meter** — toggle the taskbar meter without touching the config file.
- **Network Information** — active adapter, current sample and the detected physical interfaces.
- **Start with Windows** — toggles the Run entry, checkmark reflects the registry.
- **Check for Updates** — reports the running version (no update feed is configured in this build).
- **Exit** — stops the process and removes the tray icon.

Command line:

| Flag | Effect |
| --- | --- |
| *(none)* | Start the tray icon and the taskbar meter (single instance). |
| `--settings` | Open the settings window; focuses an existing one instead of duplicating it. |
| `--enable-startup` | Add the HKCU Run entry for this executable. |
| `--disable-startup` | Remove it. |

## Configuration

`%APPDATA%\NetPulse\config.json` is created on the first change. The background app reloads it
immediately when Settings saves, so edits take effect without restarting.

```json
{
  "visible": true,
  "show_download": true,
  "show_upload": true,
  "unit": "bits",
  "font_family": "Segoe UI Semibold",
  "font_size": 12,
  "text_color": "#FFFFFF",
  "background": "transparent",
  "background_color": "#1F1F1F",
  "background_opacity": 210,
  "position": "left_of_tray",
  "offset": 6,
  "update_interval_ms": 1000,
  "adapter": null,
  "dock_mode": "overlay",
  "theme": "system"
}
```

| Field | Values | Meaning |
| --- | --- | --- |
| `visible` | `true` / `false` | Show the meter (tray toggle writes this). |
| `show_download`, `show_upload` | `true` / `false` | Which halves to draw. Both off hides the meter. |
| `unit` | `bits` / `bytes` | Kbps/Mbps or KB/s/MB/s, scaled automatically. |
| `font_family`, `font_size` | face name, 8–24 px | Taskbar text. |
| `text_color` | `#RRGGBB` | Text colour; first run follows the light/dark taskbar theme. |
| `background` | `transparent` / `solid` | Rounded pill behind the text or fully transparent. |
| `background_color`, `background_opacity` | `#RRGGBB`, 0–255 | Pill colour and real alpha. |
| `position`, `offset` | `left_of_tray` / `right_of_tray`, px | Where to sit relative to `TrayNotifyWnd`. |
| `update_interval_ms` | 250–5000 | Sampling interval. |
| `adapter` | GUID string or `null` | Pin one interface; `null` sums all active physical ones. |
| `dock_mode` | `overlay` / `taskbar` | See [Taskbar integration](#taskbar-integration--the-honest-version). |
| `theme` | `system` / `light` / `dark` | Settings window theme (the meter has its own colours). |

## Architecture

```mermaid
flowchart LR
    subgraph P["netpulse.exe — background (single instance)"]
        direction TB
        S["sampler thread<br/>GetIfTable2 every interval"] -->|sample| H["host window thread<br/>message loop + 1 s timer"]
        H -->|render| M["meter window<br/>layered · per-pixel alpha"]
        T["tray thread<br/>tray-icon + menu"] -->|actions| H
    end
    F[("%APPDATA%\NetPulse\config.json")] <-->|read / atomic save| H
    U["netpulse.exe --settings<br/>egui/eframe process"] -->|save + notify| F
    U -->|WM_APP_CONFIG_CHANGED| H
```

| Path | Responsibility |
| --- | --- |
| `src/main.rs` | CLI flags, DPI awareness, single-instance mutex, thread startup. |
| `src/platform.rs` | Wide strings, message boxes, host-window lookup, instance guard. |
| `src/state.rs` | Shared config/sample state and window messages. |
| `src/network/adapter.rs` | `GetIfTable2` wrapper; filters real hardware interfaces, names them by GUID. |
| `src/network/monitor.rs` | Per-interface counter deltas → bytes/s; reset/disconnect handling. |
| `src/network/mod.rs` | Sampler thread: sleep, sample, publish, wake the UI. |
| `src/taskbar/mod.rs` | Hidden host window, message routing, `TaskbarCreated`, taskbar geometry. |
| `src/taskbar/overlay.rs` | Layered meter window, GDI mask → premultiplied DIB, placement and docking. |
| `src/taskbar/display.rs` | Speed formatting, meter text, colour parsing. |
| `src/tray/mod.rs` | Tray icon, menu, tooltip, registry autostart toggle. |
| `src/ui/window.rs` | egui settings window (own process), live save/reload notifications. |
| `src/startup/mod.rs` | HKCU Run entry read/write/remove. |
| `installer/netpulse.iss` | Per-user Inno Setup installer. |

**Why Settings is a separate process.** The background app is a plain Win32 message loop that costs
a few MB. Loading eframe/OpenGL permanently just for an occasional dialog would multiply that, so
`--settings` starts the UI on demand, writes the shared JSON and posts a message; the running
instance reloads it immediately.

## Performance

Measured on Windows 11 Pro 24H2 (build 26100), 1366×768 @ 100%:

| Metric | Value |
| --- | --- |
| Working set, idle | ~11 MB |
| CPU, idle (1 s sampling) | ~0.03 s per 5 s (< 1% of one core) |
| Release executable | 4.9 MiB, no runtime DLLs |
| Network traffic | none — counters are local |
| Background GPU usage | none (no eframe/OpenGL in the background process) |

The meter repaints once per sample into a ~250×20 DIB and pushes it with `UpdateLayeredWindow`; the
tray thread wakes twice a second to refresh its tooltip. Nothing busy-loops.

`cargo test` covers speed formatting and adapter filtering.

## Limitations

- **Aggregate mode counts physical interfaces only.** Windows exposes filter drivers (QoS packet
  scheduler, WFP layers) and WAN miniports as separate interfaces that see the same bytes; NetPulse
  uses the `HardwareInterface` flag, which removes that double counting. Hyper-V/vEthernet adapters
  are not hardware-flagged either, so with WSL or a VPN the physical NIC is what gets measured — pin
  the tunnel in Settings → Network if you want its counters instead.
- **Taskbar dock is unsupported.** It depends on Explorer internals and may stop working after a
  Windows update; switch back to Overlay if the meter disappears.
- **The tray icon starts in the overflow area** on Windows 11 (new icons are hidden by default).
  Drag it onto the taskbar to pin it.
- **Primary taskbar only.** The meter follows `Shell_TrayWnd`; secondary-display taskbars
  (`Shell_SecondaryTrayWnd`) are not used.
- **Vertical taskbars** rotate the text 90° (supported on Windows 10; Windows 11 no longer ships
  vertical taskbars).
- **Text transparency** is not configurable — the text is always opaque; the background opacity is
  real per-pixel alpha.
- **Check for Updates** only reports the running version; no release feed is configured.

Tested on Windows 11 24H2. Windows 10 support comes from the same Win32 APIs (`Shell_TrayWnd`,
`TrayNotifyWnd`, `GetIfTable2`, layered windows) but has not been tested on hardware here.

## License

[MIT](LICENSE)

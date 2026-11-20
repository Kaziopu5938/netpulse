# Changelog

All notable changes to NetPulse are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.1.2] - 2026-11-20

### Fixed

- **Taskbar overlay no longer disappears when the taskbar is clicked.** Both
  `Shell_TrayWnd` and the overlay meter are topmost windows, so raising the
  taskbar on a click pushed the click-through meter behind it until the next
  one-second sync made it reappear. A short watchdog timer now re-asserts the
  meter's topmost z-order without moving or resizing it, keeping the readout
  continuously visible in overlay mode. The taskbar-dock mode is unaffected.

## [1.1.1] - 2026-11-18

### Changed

- Download-first README with correct release asset names.

## [1.1.0] - 2026-11-18

### Added

- New defaults, stacked (two-line) layout, per-user MSI installer and a
  redesigned settings window with rail navigation and a live taskbar preview.

## [1.0.0] - 2026-11-18

### Added

- Initial release: live download and upload speed in the Windows 10/11 taskbar
  using a native Win32 layered overlay, tray icon with menu and live tooltip,
  configurable speed units, colours, fonts and position, and an optional
  taskbar-dock mode.

# Changelog

All notable changes to NetPulse are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.1.3] - 2026-11-20

### Security

- Launch Explorer by its absolute path from the Windows directory instead of a
  bare `explorer` name, preventing a local executable from being picked up from
  a user-writable install/portable directory.
- Remove the working directory from the DLL search order at startup.
- Fail closed when the single-instance mutex cannot be created, and bring the
  running instance to the foreground on a duplicate launch.
- Validate and bound every field loaded from the user-editable JSON config
  (lengths, ranges and file size) before it reaches Win32 APIs or unsafe code.
- The corrupt-config path no longer re-enables the autostart entry; the
  existing registry state is preserved. The autostart check now verifies the
  Run entry actually points at this executable.
- Pin dependency versions and add Dependabot, CI, CodeQL code scanning, a
  security policy and branch protection so unreviewed or failing changes cannot
  be pushed to `main`.

### Fixed

- Free both GDI font handles when the meter window is destroyed, instead of
  leaking them on every Explorer restart or dock-mode change.
- Measure text ascent with the configured font selected into the device
  context, fixing vertical misalignment/clipping with custom fonts.
- Bound and use checked arithmetic for the DIB buffer size and font scaling.
- Key sampler history by interface GUID instead of the reusable interface
  index, avoiding false speed spikes after an adapter is replugged.
- Avoid a startup panic from subtracting from `Instant` in the settings window.
- Check cross-process `MapWindowPoints` and stop posting `WM_QUIT` twice.
- Report thread-spawn failures instead of silently running without a tray icon
  or sampler.

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

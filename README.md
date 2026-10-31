# Wiggler

A small Windows utility that nudges the mouse by one pixel on a timer. Outlook and Teams treat that as activity, so the presence indicator stays green instead of turning yellow.

It uses `SendInput`, not cursor teleporting. Windows only resets the idle clock when it sees a real input event.

## Run

```powershell
cargo run
```

Release build:

```powershell
cargo run --release
```

Requires the Rust toolchain.

## Installer

A 64-bit MSI installs Wiggler to Program Files and adds a Start menu shortcut:

```cmd
build.cmd
```

The package is written to `dist\Wiggler-0.1.5-x64.msi`. Building it needs the .NET SDK; WiX is pulled in by the installer project.

## Use

1. Click **Start wiggling**.
2. Close the window to send it to the system tray. Nudges keep running.
3. Close hides it to the tray; clicking the tray icon does not bring the window back. Right-click for Show, About, or Quit.
4. Open **Options** to set the nudge interval (30 seconds to 5 minutes), skip nudges while you are using the computer, skip them when the screen is locked or a screensaver is showing, pause them after a local time or on weekends, or start Wiggler when you sign in to Windows.
5. **Help → About...** shows the CodeGator mark, version, and copyright. **File → Quit** closes the app altogether, same as tray Quit.

Start/stop is remembered. If wiggling was on when you quit or reboot, it comes back on the next launch. Sign-in start opens in the tray.

`cargo run -- --nudge-once` fires a single nudge and prints idle time before and after, which is useful for a quick sanity check.

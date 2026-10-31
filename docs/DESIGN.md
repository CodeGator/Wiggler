# Wiggler — Design

A compact Windows desk accessory. It should feel like a small lamp on the desk, not a dashboard and not a settings panel from a cloud product.

## Purpose

Wiggler periodically sends a one-pixel mouse nudge so Windows still counts the session as active. Outlook and Teams use that idle clock for the yellow Away state.

The main window has one job: make it obvious whether Wiggler is keeping the session active, and make start/stop immediate. Configuration lives in a small Options popup.

## Personality

Quiet, physical, slightly wry. Copy is short. No marketing voice, no exclamation points, no empty empty-states.

Status language is about presence, not about "productivity":

- Running: `Still here`
- Running, waiting for idle: `Waiting for idle`
- Running, screen locked or screensaver: `Screen is locked`
- Running, after the quiet-after time: `Off the clock`
- Running, weekend skip: `Weekend`
- Stopped: `Away is allowed`

The paused state uses amber on purpose — that is the color this tool exists to avoid.

## Layout

Fixed window, **380 × 500**, not resizable, centered on first open. Native title bar (dark) with close only — no minimize or maximize. Native menu bar: `File` → `Quit`, `Help` → `About...`. No sidebar, no tabs, no footer chrome.

The window close button hides Wiggler to the system tray. Nudges keep running while it is hidden. The tray is a small lamp, not a second control panel:

- Clicking the icon does not restore the window
- Menu: `Show`, `About...`, `Quit`
- `Show` restores the window
- `About...` opens the About popup (and brings the window back if it was hidden, so the popup can be seen)
- `File` → `Quit` and tray `Quit` both close the app altogether

The tray icon is green while running and amber while stopped.

Vertical rhythm, centered:

1. Wordmark `Wiggler` — small, tracked out
2. Status orb — the primary object
3. Status phrase — one line
4. Meta line — countdown or last nudge
5. Primary action — full width
6. Options — full width outline button

Padding: 28px sides, 24px top, 32px bottom. Stack gap: 16px between groups, 8px within a group.

## Options popup

A single modal window, titled `Options`, centered over the main window. Not resizable. Same olive surface as the control wells. Close with the title-bar X.

Contents, top to bottom:

1. `Nudge every` plus live value, then the interval slider. Range **30 seconds to 5 minutes**, 15-second steps.
2. Checkbox `Only when I go idle` with helper `Skip nudges while you are using the mouse or keyboard.`
3. Checkbox `Skip when the screen is locked` with helper `Do not nudge if the screen is locked or a screensaver is showing.`
4. Checkbox `No nudges after` with helper `Pause from this local time until morning.` When checked, show the live clock value on the right and a time slider (**12:00 AM–11:45 PM**, 15-minute steps). Default time **6:00 PM**.
5. Checkbox `Skip weekends` with helper `Do not nudge on Saturday or Sunday.`
6. Checkbox `Start at sign-in` with helper `Open Wiggler when you sign in to Windows.`

Wiggling on/off is remembered across launches. A sign-in start opens in the tray, not as a window.

No extra buttons. Changes apply immediately.

## About popup

A single modal window, titled `About`, centered over the main window. Not resizable. Same olive surface as Options. Close with the title-bar X.

Opened from **Help → About...** on the native menu bar, or from **About...** on the tray menu.

Contents, top to bottom, centered:

1. CodeGator portrait on a white rounded well that hugs the image (12px radius, 8px inset, portrait 120px wide). Do not stretch the well to the popup width. The source file is a JPEG with a white field; the well matches that field so edges stay clean. Prefer an SVG if one is supplied later.
2. Wordmark `Wiggler` — same treatment as the main window
3. Version number, 11px `muted`, from the Cargo package version (`0.1.5`)
4. Copyright, 11px `muted`, wrapping: `Copyright (c) 2002 - {current year} CodeGator. All rights reserved.`

No website. No extra buttons.

## Color

Drawn from a dark olive work surface, not blue-gray SaaS.

| Token | Hex | Use |
| --- | --- | --- |
| `bg` | `#14180F` | Window |
| `surface` | `#1E2418` | Control wells, Options popup |
| `line` | `#323A28` | Hairline borders |
| `text` | `#E7EDDA` | Primary copy |
| `muted` | `#8C9378` | Labels, meta, wordmark |
| `green` | `#9BE15D` | Running orb, primary button |
| `green-dim` | `#4E7A32` | Orb glow ring while running |
| `amber` | `#E6B84A` | Stopped orb |
| `amber-dim` | `#7A6424` | Orb glow ring while stopped |
| `button-text` | `#14180F` | Text on the green button |
| `danger-text` | `#E7EDDA` | Text on the stopped (outline) button |

Do not introduce a third accent. No purple, no cyan, no gradients except a soft radial fade on the orb.

## Type

**Segoe UI** only. If it is missing, fall back to the system UI font.

| Role | Size | Weight | Color | Tracking |
| --- | --- | --- | --- | --- |
| Wordmark | 11 | 600 | `muted` | +2px |
| Status | 22 | 600 | `text` | 0 |
| Meta | 12 | 400 | `muted` | 0 |
| Button | 13 | 600 | see color | 0 |
| Field label | 11 | 600 | `muted` | +0.5px |
| Field value | 12 | 400 | `text` | 0 |

No all-caps except the wordmark. Status phrase is sentence case.

## Orb

120 × 120 canvas, centered.

- Fill circle 72px, color `green` or `amber`
- 2px ring 88px, color `green-dim` or `amber-dim`
- Soft outer glow: 8 concentric rings, 4% opacity each, stepping out 3px
- While running, the ring breathes: ring diameter eases between 84px and 96px over 2.4s
- On each nudge, the fill flashes to `text` for 120ms, then returns

The orb is display-only. It is not a click target. The button is.

## Controls

**Primary button**
- Height 44, corner radius 8, full width
- Running: filled `green`, label `Stop wiggling`, text `button-text`
- Stopped: outline `line` on `surface`, label `Start wiggling`, text `text`
- Hover: 6% lighter fill (running) or `line` brightens to `#3E4634` (stopped)
- No icons on the button

**Options button**
- Same outline treatment as Start wiggling
- Label `Options`

**Interval** (Options only)
- Label `Nudge every` left, live value right (`45 sec` / `2 min` / `5 min`)
- Slider, 30–300 seconds, 15-second steps
- Track `line`, 4px tall, radius 2
- Fill from left in `green` when running, `amber` when stopped
- Thumb 14px circle, `text`

**Idle-only**
- Checkbox + label `Only when I go idle`
- Helper under it, 11px `muted`: `Skip nudges while you are using the mouse or keyboard.`
- Unchecked means a nudge fires on the interval even if the session is already active

**Skip when locked**
- Checkbox + label `Skip when the screen is locked`
- Helper: `Do not nudge if the screen is locked or a screensaver is showing.`
- Checked by default

**No nudges after**
- Checkbox + label `No nudges after`
- Helper: `Pause from this local time until morning.`
- Unchecked by default
- When checked, live value on the right (`6:00 PM`) and a slider, 00:00–23:45, 15-minute steps
- Local time. Nudges stay off until the next calendar day
- Wiggling remains on; this only pauses nudges, same as lock skipping

**Skip weekends**
- Checkbox + label `Skip weekends`
- Helper: `Do not nudge on Saturday or Sunday.`
- Unchecked by default
- Saturday and Sunday, local calendar
- Wiggling remains on; nudges resume Monday

**Start at sign-in**
- Checkbox + label `Start at sign-in`
- Helper: `Open Wiggler when you sign in to Windows.`
- Unchecked by default
- Sign-in launch hides to the tray and restores the last start/stop state

## Motion

- Orb breath: 2.4s sine, only while running
- Nudge flash: 120ms
- Countdown ticks once per second, no extra animation
- Options and About popups appear in place; no fade-in on launch

## States

| State | Orb | Status | Meta | Button |
| --- | --- | --- | --- | --- |
| Stopped | Amber, still | Away is allowed | Last nudge, or `Has not nudged yet` | Start wiggling |
| Running, idle-only, user active | Green, breathing | Waiting for idle | `Will nudge after you sit still` | Stop wiggling |
| Running, screen locked or screensaver | Green, breathing | Screen is locked | `Nudges pause until you unlock` | Stop wiggling |
| Running, after quiet-after time | Green, breathing | Off the clock | `Nudges pause until morning` | Stop wiggling |
| Running, weekend skip | Green, breathing | Weekend | `Nudges pause until Monday` | Stop wiggling |
| Running, counting down | Green, breathing | Still here | `Next nudge in M:SS` | Stop wiggling |
| Nudge just fired | Flash | Still here | `Nudged just now` | Stop wiggling |

## Content rules

- Never say "prevent detection", "bypass", or "spoof".
- Never mention HR, policy, or employers.
- One sentence of explanation is enough, and it belongs in the README, not the window.
- Numbers use a colon countdown (`1:20`), not `80 seconds remaining`.

## What this is not

Not a tray megamenu. Not a notification center. Not a chart of uptime. Not a theming playground. The main window is start/stop and Options. Quit lives on the File menu and the tray. About lives on Help and the tray. Interval, idle-only, lock skipping, quiet-after time, weekend skip, and sign-in start live in Options. The tray shows, opens About, and quits.

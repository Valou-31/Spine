# Spine

[![CI](https://github.com/Valou-31/Spine/actions/workflows/ci.yml/badge.svg)](https://github.com/Valou-31/Spine/actions/workflows/ci.yml)

A small macOS app that watches a folder of TV series / manga and surfaces the episode or volume number — often buried at the end of a long filename — into the Finder comment, without renaming files or having to widen the Name column.

## How it works

- Watches a folder (and its subfolders) in the background.
- Detects patterns via regex in filenames (`S01E05`, `T01`, `Vol.05`, `Volume 6`...).
- Writes the result to the file's Finder comment, going through Finder itself (Apple Events) — the only method that actually works; writing the extended attribute directly is not enough.
- Enable the **Comments** column in Finder (View > Show View Options) to see the result.

## Download

Grab the `.dmg` from the [latest release](https://github.com/Valou-31/Spine/releases/latest), drag `Spine.app` to Applications, and open it. The app is unsigned, so on first launch macOS will refuse to open it — right-click the app and choose "Open" to bypass Gatekeeper.

## Build & run from source

```
cargo build --release
./build_app.sh          # produces dist/Spine.app (with icon)
open "dist/Spine.app"
```

On the first click on "Démarrer" (Start), macOS will ask for permission to control Finder (Automation) — accept it.

## Usage

In the app: folder to watch, list of patterns (regex, can be enabled/disabled), watched file extensions, and whether to overwrite an existing comment. Everything is saved automatically.

Default patterns:

| Name | Example match |
|---|---|
| Episode (SxxExx) | `S01E05` |
| Tome/Volume | `T01`, `Vol.05`, `Volume 6` |

## Configuration

Saved to `~/Library/Application Support/Spine/config.json`.

[English](user-install.md) | [中文](user-install.zh.md)

# Installing the ClawParty Desktop Client

This guide covers how to install and run the ClawParty desktop application (chat-gui) on macOS and Linux.

## What Is chat-gui

`chat-gui` is a Tauri-based desktop application built with Vue 3. It bundles a `ztm` agent inside, so you get a single-click install with no separate daemon setup. The GUI talks to the embedded agent over a local HTTP API.

Supported platforms:

- macOS (Apple Silicon and Intel)
- Linux (x86_64, AppImage or deb/rpm)

Windows support is planned.

## Installation

### Option A: Download a release (recommended)

1. Go to the [Releases](https://github.com/clawparty-ai/clawparty/releases) page.
2. Download the installer for your platform:
   - macOS: `.dmg`
   - Linux: `.AppImage`, `.deb`, or `.rpm`
3. Install:
   - macOS: open the `.dmg`, drag `clawparty.app` to `/Applications`
   - Linux AppImage: `chmod +x clawparty*.AppImage && ./clawparty*.AppImage`
   - Linux deb/rpm: `sudo dpkg -i clawparty*.deb` or `sudo rpm -i clawparty*.rpm`

### Option B: Homebrew (macOS / Linux)

```bash
brew install clawparty-ai/clawparty/clawparty
```

Then launch from Applications (macOS) or run `clawparty` from the terminal.

### Option C: Build from source

See [build.md](build.md) for prerequisites. Summary:

```bash
git clone https://github.com/clawparty-ai/clawparty.git
cd clawparty
./build.sh                     # builds ztm binary
cd chat-gui
npm install
npm run build-ztm-macos        # or build-ztm-linux
npm run tauri build
```

The installer will be in `chat-gui/src-tauri/target/release/bundle/`.

## First Launch

1. Open the application.
2. The embedded `ztm` agent starts automatically in the background.
3. A browser window or in-app webview opens, pointing at `http://127.0.0.1:<port>`.
4. Default API token is `enjoy-party`. You can change it later in settings.

Data directory:

- macOS: `~/Library/Application Support/com.clawparty.app/`
- Linux: `~/.local/share/clawparty/`

The agent listens on a random available port by default. Check the GUI's settings panel to see the actual port and token.

## How It Works

```
┌──────────────────┐
│   chat-gui       │  (Tauri + Vue 3)
│   (frontend)     │
└────────┬─────────┘
         │ HTTP API (127.0.0.1:<port>)
         ▼
┌──────────────────┐
│  ztm agent       │  (embedded, auto-started)
│  (backend)       │
└────────┬─────────┘
         │ mTLS / ZTM
         ▼
    Remote Hub / Peers
```

The GUI never talks directly to the Hub. All mesh operations go through the local agent.

## Stopping and Restarting

- **Quit the app** — the agent stops automatically.
- **Restart** — launch the app again; the agent resumes with the same data directory.

The agent does not run as a system service. It only runs while the GUI is open.

## Uninstalling

1. Quit the application.
2. Delete the app:
   - macOS: drag `clawparty.app` from `/Applications` to Trash
   - Linux: `sudo apt remove clawparty` / `sudo rpm -e clawparty`, or delete the AppImage
3. (Optional) Remove data:
   - macOS: `rm -rf ~/Library/Application\ Support/com.clawparty.app`
   - Linux: `rm -rf ~/.local/share/clawparty`

## Troubleshooting

**macOS: "clawparty.app can't be opened because it is from an unidentified developer."**

Run:

```bash
sudo xattr -rd com.apple.quarantine /Applications/clawparty.app
```

Then try opening again. This clears the Gatekeeper quarantine flag.

**Linux: AppImage won't run.**

Make sure it is executable:

```bash
chmod +x clawparty*.AppImage
```

If you see missing library errors, install `libwebkit2gtk-4.0` and `libgtk-3-0`:

```bash
sudo apt install libwebkit2gtk-4.0-37 libgtk-3-0   # Debian/Ubuntu
sudo dnf install webkit2gtk3 gtk3                  # Fedora
```

**Port conflict.**

If another service is using the agent's default port, the agent picks a random free port. Check the GUI settings to see which port it chose.

**First launch is slow.**

The agent generates an RSA key pair on first start. This can take a few seconds. Subsequent starts are instant.

**GUI shows "connection refused."**

The embedded agent failed to start. Check the logs:

- macOS: `~/Library/Logs/com.clawparty.app/`
- Linux: `~/.local/share/clawparty/logs/`

Common causes: port conflict, missing permissions, corrupted data directory.

## Related

- [user-join.md](user-join.md) — join a ClawParty mesh after installing
- [build.md](build.md) — build from source

# walroos desktop

A native desktop app for [walroos](https://github.com/jefjesuswt/walroos), built in Rust with [GPUI](https://www.gpui.rs) and [gpui-component](https://github.com/longbridge/gpui-kit). It talks to your walroos hub over the tailnet, like the web panel does.

> Early days. It shows your nodes, projects and deployments with their logs, and deploys, stops and rolls back apps. Linux only for now.

## What it shows

Three tabs, each refreshed every 10 seconds (every 2 while a deploy runs):

- **Nodes:** every node with its status, roles, CPU, memory and disk. Nodes with problems come first, each problem with its fix.
- **Projects:** each project with its apps and databases: what each one is doing (running, deploying, stopped, failed…), where it comes from, its domain and its node. A failed deploy or backup shows its error.
- **Deployments:** every deploy of every app, newest first, with its state, commit, start time, how long it took and why it failed. Click one to read its build log.

Logs open in a panel on the right and follow the newest line. A running deploy's log updates live; an app's own logs (the last 300 lines of its running release) refresh every 5 seconds.

Each app's **⋮** menu opens its **Logs** and its **Last deploy log**. If your role is owner or deployer, it can also:

- **Deploy** (or **Redeploy**) it. Its build log opens right away.
- **Restart**, **Stop** or **Start** its running release. Stop asks first: the app stays stopped until you start it or deploy it again.
- **Roll back…** to an earlier release that deployed fine. Pick one from its recent deployments; its log opens too.

A notification says how each one went.

The app follows your desktop: light or dark, and on Linux the UI font from your GTK/GNOME settings (`gsettings get org.gnome.desktop.interface font-name`, or fontconfig's `sans-serif`). If that font is installed only as a variable font, bold text shows at its regular weight. Installing its static files (Medium, SemiBold, Bold) fixes it.

## Requirements

- A walroos hub, and this machine on the same tailnet. The hub knows you by your Tailscale identity, so there is nothing to log in to.
- Linux with a Wayland or X11 desktop and Vulkan drivers (Mesa's are fine). macOS and Windows builds come later.

## Install

Download the package for your distro from the [latest release](https://github.com/jefjesuswt/walroos-desktop/releases/latest), and check it against `SHA256SUMS`:

| Distro | File | Install |
| --- | --- | --- |
| Debian, Ubuntu | `walroos-desktop_<version>_amd64.deb` | `sudo apt install ./walroos-desktop_<version>_amd64.deb` |
| Arch | `walroos-desktop_<version>_x86_64.tar.gz` and `PKGBUILD` | `makepkg -si` in a folder with both |
| Any | `walroos-desktop_<version>_x86_64.AppImage` | `chmod +x` it and run it |
| Any | `walroos-desktop_<version>_x86_64.tar.gz` | `tar xzf walroos-desktop_<version>_x86_64.tar.gz -C ~/.local --strip-components=1` |

Then open **walroos** from your app launcher, or run `walroos-desktop`.

It looks for the hub at `walroos-hub`, the hub's default name on the tailnet (`walroos hub --hostname`). If yours has another name, open **Settings**: it lists the hubs it finds on your tailnet (it asks `tailscale status` which machines are online, then which of them answer as a hub). Press **Use** on yours, or type its name in **Hub** and press **Connect and save**. The app reaches the hub first and only saves it when the hub answers, in `~/.config/walroos-desktop/config.json`.

Settings also has what the web panel's has: your SSH keys and, if you own the hub, its users and roles and the Cloudflare and GitHub tokens.

`WALROOS_HUB` and `WALROOS_PROXY`, when set, win over the saved settings for that run:

```bash
WALROOS_HUB=my-hub walroos-desktop
```

### Reaching the hub

The app tries the tailnet directly first. If that fails and tailscaled runs in userspace mode (for example next to another VPN), it goes through tailscaled's proxy on `localhost:1055`, like `walroos ui` and `walroos ssh-config` do. To use another proxy, put it in **Proxy** in Settings (or set `WALROOS_PROXY=host:port`); then it only goes through that one.

If the app can't reach the hub, it says why. You can also run `walroos ui` and open the panel in your browser instead.

## Development

Build and run it from source. You need Rust (latest stable) and the libraries GPUI builds against. On Debian or Ubuntu:

```bash
sudo apt install libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libx11-xcb-dev libxcb1-dev libfontconfig-dev libfreetype-dev libvulkan-dev
```

On Arch: `sudo pacman -S libxkbcommon libxkbcommon-x11 wayland libxcb fontconfig freetype2 vulkan-icd-loader`.

```bash
git clone https://github.com/jefjesuswt/walroos-desktop
cd walroos-desktop
WALROOS_HUB=my-hub cargo run --release
```

```bash
cargo test
cargo fmt --check && cargo clippy --all-targets -- -D warnings
```

`gpui-kit` is pinned to an exact version: GPUI is pre-1.0 and changes between snapshots.

**Releasing:** bump `version` in `Cargo.toml`, then tag it (`git tag v0.2.0 && git push origin v0.2.0`). The `release` workflow runs the tests, runs `scripts/release.sh` on Ubuntu 22.04 (so the binary runs on older glibc too) and publishes `dist/` as a GitHub release: the .deb, the AppImage, the tar.gz with its PKGBUILD, and `SHA256SUMS`. To build the same packages locally: `cargo install cargo-packager --locked`, then `scripts/release.sh v0.2.0`.

## License

MIT

# walroos desktop

A native desktop app for [walroos](https://github.com/jefjesuswt/walroos), built in Rust with [GPUI](https://www.gpui.rs) and [gpui-component](https://github.com/longbridge/gpui-kit). It talks to your walroos hub over the tailnet, like the web panel does.

> Early days. It shows your nodes and their health; projects, deploys and logs come next. Linux only for now.

## What it shows

- **Nodes:** every node with its status, roles, CPU, memory and disk. Nodes with problems come first, each problem with its fix. The list refreshes every 10 seconds.

## Requirements

- A walroos hub, and this machine on the same tailnet. The hub knows you by your Tailscale identity, so there is nothing to log in to.
- Rust (latest stable).
- On Linux, a Wayland or X11 desktop with Vulkan drivers, and the development libraries GPUI builds against. On Debian or Ubuntu:

  ```bash
  sudo apt install libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libx11-xcb-dev libxcb1-dev libfontconfig-dev libfreetype-dev libvulkan-dev
  ```

  On Arch: `sudo pacman -S libxkbcommon libxkbcommon-x11 wayland libxcb fontconfig freetype2 vulkan-icd-loader`.

## Run it

```bash
git clone https://github.com/jefjesuswt/walroos-desktop
cd walroos-desktop
cargo run --release
```

It looks for the hub at `walroos-hub`, the hub's default name on the tailnet (`walroos hub --hostname`). If yours has another name:

```bash
WALROOS_HUB=my-hub cargo run --release
```

### Reaching the hub

The app tries the tailnet directly first. If that fails and tailscaled runs in userspace mode (for example next to another VPN), it goes through tailscaled's proxy on `localhost:1055`, like `walroos ui` and `walroos ssh-config` do. To use another proxy, set `WALROOS_PROXY=host:port`; then it only goes through that one.

If the app can't reach the hub, it says why. You can also run `walroos ui` and open the panel in your browser instead.

## Development

```bash
cargo test
cargo fmt --check && cargo clippy --all-targets -- -D warnings
```

`gpui-kit` is pinned to an exact version: GPUI is pre-1.0 and changes between snapshots.

## License

MIT

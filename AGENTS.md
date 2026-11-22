# walroos-desktop — notes for coding agents

Native desktop app for [walroos](https://github.com/jefjesuswt/walroos), in Rust with GPUI through `gpui-kit` (gpui-component). A client of the hub's HTTP API (`/api/*`), nothing more: no new endpoints in the hub for it.

- The owner speaks Spanish: talk to them in Spanish. Code, UI copy, commits and code comments are in English.
- **The contract** is `web/src/types.ts` and `web/src/api.ts` in the walroos repo. `src/api.rs` copies only the fields the app shows.
- **Reaching the hub** (`src/hub.rs`) mirrors `cmd/walroos/hubconn`: directly, else through tailscaled's proxy on `localhost:1055`. The hub knows you by your tailnet identity; there is no login.
- **Blocking HTTP (`ureq`) on `cx.background_spawn`**, results back with `cx.spawn` + `update`. GPUI's executor isn't tokio.
- **gpui-kit is pinned exactly.** GPUI breaks between snapshots; upgrade it in a PR of its own.
- **Linux first.** macOS and Windows come later.
- **Pages** are a `page::Spec` (title, fetch, body) shown by the generic `Page`, which polls and shows errors; `shell.rs` holds the tabs. All pages share one `link::Link` to the hub.
- **States and labels** of apps and databases come from `status.rs`, which mirrors `web/src/status.ts`: a service shows the same state as in the web panel.
- Keep files under ~300 lines; one page per file, its pieces apart (`nodes.rs`, `node_card.rs`).
- In test modules import names explicitly: `use gpui_kit::*` brings GPUI's own `#[test]`, which clashes with Rust's.
- Mark deliberate shortcuts with a `// ponytail:` comment that names the limit.
- Commits: conventional (`feat:`, `fix:` …), on a branch, not `main`.
- **The repo is public.** Never write the owner's machine names, hostnames, IPs or domains into tracked files, commits or PRs. Use stand-ins (`walroos-hub`, `prod-1`, `100.64.0.x`).

## Commands

```bash
cargo run                                   # WALROOS_HUB=<hub> WALROOS_PROXY=<host:port> to override
cargo test
cargo fmt --check && cargo clippy --all-targets -- -D warnings
```

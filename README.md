# SigmaOS Update System

SigmaOS rolling release update manager — **experimental prototype**.

> **Status:** this crate is *not* the updater that ships in the ISO.
> The installed system uses the self-contained `sigmaos-update` shell
> script (pacman `-Syu` + btrfs snapshot hooks + dotfile/theme sync),
> shipped from the SigmaOS-ISO repository. This Rust component explores
> a channel/pocket-based updater against a future `updates.sigmaos.org`
> service that does not exist yet.

## What exists today

- `check` / `update` / `rollback` / `status` / `config` CLI (clap)
- Channel model (`stable`/`beta`/`nightly`) mapped onto release-pocket
  names for a future pocket-aware backend
- Update state + config persistence under `~/.config/sigmaos/`
- `UpdateNotifier` hook for desktop notifications

## Known gaps (do not rely on this for real updates)

- `fetch_updates` calls `https://updates.sigmaos.org/...` — no such
  service exists; `check` returns an empty list today.
- `verify_package` only checks that the signature string and archive
  are non-empty — **no cryptographic verification is implemented**.
- `install_package` shells out to `sigma-pkg install`, which is not a
  shipped component.
- `rollback` swaps a recorded version string; it does not restore
  package or filesystem state.

## Build & Test

```sh
cargo build
cargo test
```

## License

GPL-3.0-or-later

## See Also

- [SigmaOS-ISO](../SigmaOS-ISO/README.md) — ships the actual `sigmaos-update` script
- [SigmaOS-pockets](../SigmaOS-pockets/README.md) — release pocket model this crate's channels map onto
- [SigmaOS-pkg-solver](../SigmaOS-pkg-solver/README.md) — dependency solver for package-level updates
- [SigmaOS-ab-boot](../SigmaOS-ab-boot/README.md) — A/B slot state machine (future rollback path)

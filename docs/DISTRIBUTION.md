# Distribution — how people install keinontolibrary

Ideation for the install channels, with effort/reward and a recommended order. Two
artifacts ship separately:

- **the library** (`keinontolibrary-core` + friends) — for Rust programs;
- **the CLI** (`keinontolibrary`) — a standalone binary for everyone else.

The **HTTP service** ships as a container image, not a package (see
[http-service](guides/http-service.md)). One constant across every channel: the **data
artifact is not redistributable yet** (`LICENSING.md`), so packages ship code only and
build/fetch the artifact at first run — see [§ The data problem](#the-data-problem).

## Recommended order

1. **crates.io** (library + CLI) — the native Rust channel; unlocks `cargo install` and
   `cargo binstall`. Lowest effort, highest reach for the primary audience.
2. **GitHub Releases with prebuilt binaries** — cross-platform tarballs via
   `cargo-dist`; the substrate Homebrew/Scoop/binstall all draw from.
3. **Homebrew** (macOS/Linux) — a tap, then ideally homebrew-core.
4. **Everything else** (apt/deb, Nix, Scoop, AUR, Docker, WASM/npm) — as demand appears.

---

## Channels

### crates.io — `cargo install` / `cargo binstall`
- **What**: `cargo add keinontolibrary-core`; `cargo install keinontolibrary-cli`.
- **Effort**: low. Publish the workspace crates in dependency order
  (core → rules → data → cli/server). Needs each crate's `description`, `keywords`,
  `categories`, `readme`, `license` — mostly present. `cargo-binstall` works for free once
  GitHub Release binaries exist (channel 2).
- **Blocker**: the CLI/server depend on the artifact at runtime; the *library* crates can
  publish immediately. Gate CLI publish on the data story.

### GitHub Releases — prebuilt binaries (`cargo-dist`)
- **What**: tagged releases attach `keinontolibrary-<version>-<target>.tar.gz` for
  macOS (arm64/x64), Linux (x64/arm64 musl — static, like the container), Windows x64.
- **Effort**: low–medium. `cargo dist init` generates a release workflow that builds the
  matrix on tag push and uploads artifacts + checksums + an installer script
  (`curl … | sh`). It can also emit a Homebrew formula and a Scoop manifest, so this one
  step seeds channels 3 and parts of 4.
- **Reward**: high — no-toolchain installs, and the source of truth for downstream
  package managers.

### Homebrew (macOS + Linux)
- **What**: `brew install keinontolibrary`.
- **Effort**: medium. Start with a **tap** (`timokoola/homebrew-tap`) — cargo-dist can
  generate and push the formula automatically each release. Graduate to **homebrew-core**
  later (needs notability + stable release cadence; their audit is strict, and a formula
  that downloads data at runtime needs care).
- **Note**: Homebrew prefers building from source or a release tarball; the static-musl
  Linux binary and a macOS universal binary from channel 2 both work.

### apt / .deb (Debian/Ubuntu)
- **What**: `apt install keinontolibrary` (from a hosted repo) or a downloadable `.deb`.
- **Effort**: medium (own repo) to high (Debian official). Two realistic routes:
  - **`cargo-deb`** to produce a `.deb` per release (attach to GitHub Releases); users
    `dpkg -i`. Cheapest.
  - **A hosted APT repo** (e.g. an `aptly`/`deb-s3` bucket, or Cloudsmith/PackageCloud)
    for true `apt install`. More upkeep (GPG signing, repo metadata).
  - Debian/Ubuntu *official* archives are high-effort (packaging policy, a maintainer,
    vendoring the Rust dep tree) — defer unless there's real demand.
- **Constraint**: a distro package really wants the data bundled; until the corpus license
  is settled, a `.deb` would ship the CLI and fetch/build the artifact on first run — or
  ship only the rule-engine subset (no corpus).

### Nix / nixpkgs
- **What**: `nix run`, or a flake + an eventual nixpkgs entry.
- **Effort**: low for a `flake.nix` in-repo (great for contributors and reproducible
  builds); medium for nixpkgs submission. Rust + crates map cleanly via `crane`/
  `buildRustPackage`. Good fit for the audience; low priority but cheap to start.

### Scoop / WinGet (Windows)
- **What**: `scoop install keinontolibrary` / `winget install`.
- **Effort**: low (Scoop manifest, cargo-dist can emit it) to medium (WinGet PR). Do
  alongside channel 2; small Windows audience but nearly free.

### AUR (Arch)
- **What**: `yay -S keinontolibrary-bin`.
- **Effort**: low — a `-bin` PKGBUILD pointing at the GitHub Release binary. Community-
  maintainable. Nice-to-have.

### Docker / GHCR (the service)
- **What**: `docker pull ghcr.io/timokoola/keinontolibrary`.
- **Effort**: low. The `Dockerfile` already builds a <10 MB image; add a release workflow
  step to build+push to GHCR on tag. This is the install path for the **server**, not the
  library/CLI.

### WASM / npm (future)
- **What**: `npm i keinontolibrary` — the FFI crate's `wasm` target compiled to a small
  module for JS/edge use (the roadmap's Cloudflare Workers direction).
- **Effort**: medium–high; the FFI scaffold exists but the WASM bindings + packaging are
  unbuilt. Defer to a dedicated effort.

---

## Packaging the data (cross-cutting)

The artifact is cleared for redistribution (`LICENSING.md`), so a channel may **bundle it
directly**. The remaining trade-off is package size, so a channel can alternatively use:

1. **Fetch-on-first-run** — the package ships code; on first use it downloads the prebuilt
   artifact from a release asset. Smallest install.
2. **Rule-engine-only build** — ship without the corpus lookup; the rule engine + registry
   still cover the vast majority of forms (lower accuracy on the long tail). A feature flag
   could select this.
3. **Build-it-yourself** — the package provides `keinontolibrary ingest`; the user
   supplies sources. Fine for developers, poor for end users.

**Recommendation**: resolve the `LICENSING.md` question first; it unblocks fetch-on-first-
run (option 1), the best experience, for *all* channels at once. In the meantime publish
the **library crates** (no data) and the **container** (operator supplies data) — both are
shippable now.

# Publishing runbook

Release steps are run manually. The data-backed artifact is cleared for redistribution
(`LICENSING.md`), so packages bundle it.

Prerequisite for every data-bearing channel: `data/artifact/keinontolibrary.bin` +
`data/overlay.jsonl` exist locally (build them per `guides/build-artifact.md`).

## 1. GitHub release assets (needed by Homebrew)

Publish the artifact + overlay to the release and fill the formula checksums:

```sh
packaging/homebrew/update-shas.sh v0.1.0
```

## 2. PyPI — `pip install keinontolibrary` / `uvx keinontolibrary`

Native PyO3 wheels (arch-specific) + sdist, built and uploaded locally:

```sh
# one-time: rustup target add x86_64/aarch64 -unknown-linux-gnu + -apple-darwin
bindings/python/build-wheel.sh              # smoke-test a local wheel first
MATURIN_PYPI_TOKEN=pypi-XXXX bindings/python/publish.sh
```

`publish.sh` builds a macOS universal2 wheel, Linux manylinux wheels (x86_64 + aarch64, via
`zig` cross-compile — no Docker), and an sdist, then `maturin upload`s them. Use `--dry-run`
to build without uploading. Needs a PyPI API token.

Windows wheels are not built here; Windows users fall back to the sdist (needs a Rust
toolchain).

## 3. Homebrew — `brew install timokoola/tap/keinontolibrary`

After step 1, publish the formula to a personal tap (see
`packaging/homebrew/README.md`). homebrew-core is deferred (notability + data-asset audit).

## 4. crates.io (optional, unblocked by the license clearance)

The library crates can publish immediately; the CLI/server bundle or locate the artifact.
Publish in dependency order: core → rules → data → cli/server. (`cargo publish -p …`.)

---

Order for a fresh version bump: build artifact → step 1 → step 2 → step 3 → (step 4). Bump
the version in `bindings/python/pyproject.toml` + `Cargo.toml` workspace and re-tag first.

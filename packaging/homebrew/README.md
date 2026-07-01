# Homebrew packaging

`keinontolibrary.rb` installs the CLI. It builds the binary from the tagged source tarball
(needs Rust) and installs the **data-backed artifact + overlay** from release assets, then
wraps the binary so `KEINONTO_ARTIFACT` / `KEINONTO_OVERLAY` default to the installed data.

## One-time release prep

The artifact can't be rebuilt without the private corpus, so it ships as a release asset:

```sh
# publishes data/artifact/keinontolibrary.bin + data/overlay.jsonl to the v0.1.0 release
# and fills the sha256 placeholders in keinontolibrary.rb
packaging/homebrew/update-shas.sh v0.1.0
```

Prerequisite: the `LICENSING.md` corpus-cleared change is merged (publishing the artifact is
public redistribution of the corpus). Only run this once you intend that.

## Ship via a personal tap (recommended first)

homebrew-core has a **notability bar** (well-known / widely-used projects) that a brand-new
library will not clear yet, and it is unenthusiastic about formulae that pull a multi-MB data
blob from a release asset. A personal tap avoids both and is fully under your control:

```sh
# create the tap repo once
gh repo create timokoola/homebrew-tap --public -d "Homebrew tap for keinontolibrary"
git clone https://github.com/timokoola/homebrew-tap && cd homebrew-tap
mkdir -p Formula && cp ../keinontolibrary/packaging/homebrew/keinontolibrary.rb Formula/
git add Formula/keinontolibrary.rb && git commit -m "keinontolibrary 0.1.0" && git push
```

Users then:

```sh
brew install timokoola/tap/keinontolibrary
```

Validate before pushing: `brew install --build-from-source ./Formula/keinontolibrary.rb`
then `brew test keinontolibrary` and `brew audit --strict --new keinontolibrary`.

## homebrew-core (later)

Same formula format. Submit as a PR to `Homebrew/homebrew-core` once the project is notable
and you're ready for their audit. Expect pushback on the data-asset resource; the fallback is
a rule-engine-only build (no corpus) that builds entirely from source. Revisit when demand is
there — the tap covers `brew install` in the meantime.

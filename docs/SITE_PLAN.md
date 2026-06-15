# Plan: keinonto.com — docs site, front page, use-case guides, explainer video

Goal: a public home for the open-source library at **keinonto.com**, served from this
repo via **GitHub Pages**, plus task-oriented documentation and a short explainer video.
Web *surfaces* (the declension app / pSEO pages) live elsewhere; this plan is only the
**library's** site and docs.

Status of prerequisites (all met): repo is secret-clean, CI green (fmt+clippy+test, MSRV
1.85, cargo-audit), v0.1.0 tagged, 100% coverage. The corpus/artifact stay unpublished
(see `LICENSING.md`); the site ships only source, the rule-generated content export, and
prose — none of it license-encumbered.

---

## 1. Tooling — mdBook

Use **mdBook** (the Rust-project standard) to render `docs/` to a static site:

- Markdown in, static HTML out; trivial to host on Pages; search built in.
- Source guides double as repo docs (no duplication) and as the site.
- A landing page via a theme override or an `index.html` redirect to the book intro.

Layout:

```
docs/
  book.toml            # mdBook config (title, custom domain, repo link, edit-on-GitHub)
  SUMMARY.md           # the table of contents (chapter order)
  intro.md             # front page (the pitch — see §2)
  guides/              # the use-case docs (§3) — already written, real content
  reference/           # generated rustdoc link + the irregulars showcase
```

Build/deploy: a `pages.yml` workflow on push to `main` runs `mdbook build docs` and
publishes `docs/book/` to Pages. Custom domain `keinonto.com` via a `CNAME` file +ali the
repo Pages settings (DNS: `CNAME` to `timokoola.github.io`, or apex `A`/`AAAA` to
GitHub's Pages IPs). Coexistence with any existing keinonto.com use: put the library site
on a path or subdomain (e.g. `keinonto.com/lib` or `lib.keinonto.com`) if the apex is
taken by the app — decide when wiring DNS.

Alternative considered: plain hand-written HTML (more design control, more upkeep) or
Zola. mdBook wins on lowest-maintenance + docs-as-code. Revisit only if the front page
needs heavy custom design.

---

## 2. Front page (`intro.md`)

A developer-first landing that answers "what, why, how fast can I use it" above the fold.

- **One-liner + live example**:
  `decline("hevonen", Plural, Inessive) -> ["hevosissa"]`
- **Three claims, each one line**: declines *every* Kotus 2024 nominal (100% coverage,
  Voikko-verified); rules + corpus + registry, not a lookup table; embeddable Rust, CLI,
  HTTP, <10 MB container.
- **The "break-it" demo** (the hook): a small grid of the words people try to trip it on
  — `kuka → kenet`, `kahdeskymmenes → kahdennenkymmenennen`, `jokin → jossakin`,
  `parfait → parfait'ta`, `sakset` (no singular). Links to the relevant guide/showcase.
- **Copy-paste quickstart**: the three smallest on-ramps (Cargo dep, `cargo install` CLI,
  `docker run`) — pulled from the guides.
- **Footer**: crates.io (when published), GitHub, license (MIT code / CC BY 4.0 data),
  attribution to Kotus + Voikko.

Tone: precise, confident, no marketing fluff — matches the codebase.

---

## 3. Use-case documentation (`docs/guides/`)

Task-oriented, one page per "I want to…". Written in this PR (real, accurate to v0.1.0):

| Guide | Audience | "I want to…" |
| --- | --- | --- |
| `embed-rust.md` | Rust devs | add the crate and decline words in my program |
| `cli.md` | anyone | look up a declension from the terminal / a script |
| `http-service.md` | ops | run the declension service / container |
| `build-artifact.md` | maintainers | rebuild the data artifact from sources |
| `contributing.md` | contributors | fix a wrong declension the right way |

Each guide: the goal, the minimal working snippet, the common variations, and the gotchas
(ambiguity, defective forms, overlay precedence). Cross-linked; the front page lifts the
quickstarts from them.

A `reference/irregulars.md` showcase renders highlights from `data/content/irregulars.json`
(the families + war stories) — the same data the app's pSEO uses, here as a human-readable
"why Finnish is hard" page. Strong shareable content; links back to keinonto.com app pages.

---

## 4. Explainer video (Remotion, for a younger audience)

A 45–60 s programmatic video (Remotion = React → MP4), embeddable on the front page and
shareable vertically (9:16) for social. Lives in a small `video/` workspace (its own
`package.json`; not part of the Rust build).

**Narrative** (show, don't lecture):
1. Hook (0–5 s): "Finnish has 15 cases. One word, ~30 forms." `talo` fans out into its
   paradigm, animated.
2. The trap (5–20 s): type the nerd-test words; a naive table fails (`*kahdeksaskymmenettä`
   in red), keinontolibrary nails them (`kahdennenkymmenennen` in green). Rapid cuts.
3. The why (20–40 s): three quick cards from the war stories — `vaa'an` but `vaaoissa`;
   `afääriä` but `countrya`; `sakset` has no singular. Big type, minimal words.
4. The payoff (40–55 s): "100% of Finnish nouns. Open source." Logo, keinonto.com, GitHub.

**Production**: data-driven — Remotion reads `data/content/irregulars.json` so the on-screen
forms are the real, verified ones (and stay correct if regenerated). Finnish + English
caption tracks. Bold, high-contrast, motion-heavy (TikTok/Shorts idiom), not corporate.
Music: royalty-free, licensed.

**Scope note**: storyboard + a 3-scene proof-of-concept first; full render once the
front page exists. This is the lightest-committed item — plan now, build after the site.

---

## 5. Rollout order

1. **This PR**: `docs/guides/*` (real content) + `SITE_PLAN.md`. Docs usable immediately
   from the repo.
2. mdBook scaffold (`book.toml`, `SUMMARY.md`, `intro.md`) + `pages.yml` deploy workflow.
3. Enable Pages, wire `keinonto.com` DNS + `CNAME` (coordinate with the app's use of the
   domain).
4. `reference/irregulars.md` showcase generated from the content export.
5. Remotion storyboard → PoC → full video, embedded on the front page.
6. (Parallel, independent) publish to crates.io once the data-license question in
   `LICENSING.md` is settled for any bundled data — the code crates can publish sooner.

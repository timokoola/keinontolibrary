#!/usr/bin/env node
// Pull "wrong form" reports from the shared reports-api and open one GitHub issue each,
// then mark them exported so they are not imported twice.
//
// Env:
//   REPORTS_API_URL       base URL of the reports-api (required)
//   REPORTS_ADMIN_TOKEN   bearer token for the reports-api (required)
//   GITHUB_TOKEN          token with `issues: write` (required unless DRY_RUN)
//   GITHUB_REPOSITORY     "owner/repo" (auto-set by GitHub Actions; required otherwise)
//   PROJECT               optional project filter (e.g. "humalapaikallissija")
//   ISSUE_LABELS          optional comma-separated labels (default "accuracy-report")
//   DRY_RUN               "1" to print what would happen without writing anything
//
// Usage: node scripts/import-reports.mjs

const API = requireEnv('REPORTS_API_URL').replace(/\/+$/, '');
const ADMIN_TOKEN = requireEnv('REPORTS_ADMIN_TOKEN');
const DRY_RUN = process.env.DRY_RUN === '1';
const PROJECT = process.env.PROJECT || '';
const LABELS = (process.env.ISSUE_LABELS || 'accuracy-report')
  .split(',')
  .map((s) => s.trim())
  .filter(Boolean);

const GH_TOKEN = process.env.GITHUB_TOKEN || '';
const REPO = process.env.GITHUB_REPOSITORY || '';
if (!DRY_RUN && (!GH_TOKEN || !REPO)) {
  fail('GITHUB_TOKEN and GITHUB_REPOSITORY are required unless DRY_RUN=1');
}

main().catch((e) => fail(e?.stack || String(e)));

async function main() {
  const reports = await fetchNewReports();
  console.log(`Fetched ${reports.length} new wrong-form report(s)${PROJECT ? ` for ${PROJECT}` : ''}.`);

  let created = 0;
  let failed = 0;
  for (const r of reports) {
    const { title, body, labels } = renderIssue(r);
    if (DRY_RUN) {
      console.log(`\n--- DRY RUN issue for report ${r.id} [${labels.join(', ')}] ---\n${title}\n${body}\n`);
      continue;
    }
    try {
      const issue = await createIssue(title, body, labels);
      await markExported(r.id, issue.html_url);
      console.log(`✓ ${r.id} -> ${issue.html_url}`);
      created++;
    } catch (e) {
      console.error(`✗ ${r.id}: ${e?.message || e}`);
      failed++;
    }
  }

  console.log(
    DRY_RUN
      ? `\nDRY RUN complete: ${reports.length} report(s) would be imported.`
      : `\nDone: ${created} issue(s) created, ${failed} failed.`
  );
  if (failed > 0) process.exit(1);
}

async function fetchNewReports() {
  const url = new URL(`${API}/reports`);
  url.searchParams.set('status', 'new');
  url.searchParams.set('verdict', 'wrong');
  if (PROJECT) url.searchParams.set('project', PROJECT);
  url.searchParams.set('limit', '500');
  const res = await fetch(url, { headers: { Authorization: `Bearer ${ADMIN_TOKEN}` } });
  if (!res.ok) throw new Error(`reports-api GET ${res.status}: ${await res.text()}`);
  const data = await res.json();
  return Array.isArray(data.reports) ? data.reports : [];
}

function renderIssue(r) {
  const code = (v) => (v ? `\`${v}\`` : "_(n/a)_");
  const meta = r.meta ? "```json\n" + JSON.stringify(r.meta, null, 2) + "\n```" : "_(none)_";
  // A "missing word" suggestion (from /puuttuvat) is also stored as verdict=wrong, but it is
  // a form for a word the engine can't yet decline — not a correction to a shown form.
  const isMissing = Boolean(r.meta && r.meta.missing);
  const fields = [
    [isMissing ? "word (no form yet)" : "shown word/form", code(r.word)],
    ["lemma", code(r.lemma)],
    ["suggested form", r.correction ? code(r.correction) : "_(none given)_"],
    ["verdict", r.verdict],
    ["project", r.project],
    ["country", r.country || "_(n/a)_"],
    ["reported at", r.created_at],
    ["report id", code(r.id)],
  ];
  const title = isMissing
    ? `Missing form: ${r.word}${r.correction ? ` → ${r.correction}` : ""} (${r.project})`
    : `Accuracy: "${r.word}" reported wrong (${r.project})`;
  const intro = isMissing
    ? `A user suggested the plural inessive for **${r.word}**, a word the engine cannot yet decline (flagged via \`/puuttuvat\`).`
    : `A user reported a form as **wrong** via \`${r.project}\`.`;
  const body = [
    intro,
    "",
    "| field | value |",
    "| --- | --- |",
    ...fields.map(([k, v]) => `| ${k} | ${v} |`),
    "",
    "**meta**",
    meta,
    "",
    "<sub>Imported automatically from the keinonto-web reports-api.</sub>",
  ].join("\n");
  const labels = isMissing ? [...new Set([...LABELS, "missing-form"])] : LABELS;
  return { title, body, labels };
}

async function createIssue(title, body, labels) {
  // Try with labels; if any label doesn't exist GitHub returns 422 — retry without them.
  const payload = { title, body, labels };
  let res = await ghPost(`/repos/${REPO}/issues`, payload);
  if (res.status === 422 && labels.length) {
    delete payload.labels;
    res = await ghPost(`/repos/${REPO}/issues`, payload);
  }
  if (!res.ok) throw new Error(`GitHub POST issues ${res.status}: ${await res.text()}`);
  return res.json();
}

function ghPost(path, payload) {
  return fetch(`https://api.github.com${path}`, {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${GH_TOKEN}`,
      Accept: 'application/vnd.github+json',
      'X-GitHub-Api-Version': '2022-11-28',
      'User-Agent': 'keinontolibrary-import-reports',
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(payload),
  });
}

async function markExported(id, issueUrl) {
  const res = await fetch(`${API}/reports/update`, {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${ADMIN_TOKEN}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ id, status: 'exported', issue_url: issueUrl }),
  });
  if (!res.ok) throw new Error(`reports-api update ${res.status}: ${await res.text()}`);
}

function requireEnv(name) {
  const v = process.env[name];
  if (!v) fail(`Missing required env var ${name}`);
  return v;
}

function fail(msg) {
  console.error(`error: ${msg}`);
  process.exit(1);
}

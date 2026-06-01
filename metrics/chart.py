#!/usr/bin/env python3
"""Render a LinkedIn-friendly accuracy graphic from metrics.json.

Usage: <venv>/bin/python metrics/chart.py
Reads  metrics/metrics.json, writes metrics/keinontolibrary-accuracy.png
"""
import json
import pathlib

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.gridspec import GridSpec

HERE = pathlib.Path(__file__).parent
data = json.loads((HERE / "metrics.json").read_text())

CASES = [
    "nominative", "genitive", "partitive", "accusative", "inessive", "elative",
    "illative", "adessive", "ablative", "allative", "essive", "translative",
    "abessive", "comitative", "instructive",
]

def acc(rows, number, case):
    for r in rows:
        if r["number"] == number and r["case"] == case:
            return (100.0 * r["matched"] / r["supported"]) if r["supported"] else None
    return None

rows = data["by_case_number"]
sg = [acc(rows, "singular", c) for c in CASES]
pl = [acc(rows, "plural", c) for c in CASES]

overall = 100.0 * data["engine_matched_forms"] / data["engine_supported_forms"]
coverage = 100.0 * data["engine_supported_forms"] / data["total_attested_forms"]
nouns = data["kotus_noun_lemmas"]
forms = data["engine_supported_forms"]

# ---- colours ----
BLUE = "#0053a5"     # Finnish blue (singular)
LBLUE = "#5aa0e6"    # plural
INK = "#11203a"
GREY = "#5b6b7a"
BG = "#ffffff"

plt.rcParams.update({
    "font.family": "DejaVu Sans",
    "font.size": 12,
    "axes.edgecolor": "#d4dae3",
    "text.color": INK,
    "axes.labelcolor": INK,
})

fig = plt.figure(figsize=(12.8, 8.0), dpi=150, facecolor=BG)
gs = GridSpec(1, 2, width_ratios=[1.55, 1.0], wspace=0.12,
              left=0.13, right=0.97, top=0.80, bottom=0.09)

# ---- title block ----
fig.text(0.04, 0.945, "Declining every Finnish noun — in Rust",
         fontsize=27, fontweight="bold", color=INK)
fig.text(0.04, 0.895,
         "keinontolibrary: a rule engine for Kotus noun declension, "
         "scored against a reference corpus",
         fontsize=13.5, color=GREY)

# ---- left: per-case accuracy (singular vs plural) ----
ax = fig.add_subplot(gs[0])
y = range(len(CASES))
h = 0.38
ax.barh([i + h / 2 for i in y], [s or 0 for s in sg], height=h, color=BLUE,
        label="singular", zorder=3)
ax.barh([i - h / 2 for i in y], [p or 0 for p in pl], height=h, color=LBLUE,
        label="plural", zorder=3)
ax.set_yticks(list(y))
ax.set_yticklabels([c.capitalize() for c in CASES], fontsize=11)
ax.invert_yaxis()
ax.set_xlim(90, 100)
ax.set_xlabel("agreement with reference corpus (%)", fontsize=11, color=GREY)
ax.set_title("Accuracy by case", fontsize=14, fontweight="bold", loc="left", pad=10)
ax.xaxis.grid(True, color="#eef1f5", zorder=0)
ax.set_axisbelow(True)
for spine in ("top", "right", "left"):
    ax.spines[spine].set_visible(False)
ax.tick_params(length=0)
# value labels on the plural bar (the lower/limiting one) when notably below 99
for i, p in enumerate(pl):
    if p is not None and p < 98.0:
        ax.text(p - 0.2, i - h / 2, f"{p:.0f}", va="center", ha="right",
                fontsize=8.5, color="white", fontweight="bold")
ax.legend(loc="lower right", frameon=False, fontsize=11, ncol=2,
          bbox_to_anchor=(1.0, 1.0))

# ---- right: headline stats ----
axr = fig.add_subplot(gs[1])
axr.axis("off")
axr.text(0.0, 0.99, f"{overall:.1f}%", fontsize=68, fontweight="bold", color=BLUE,
         va="top")
axr.text(0.02, 0.78, "of generated forms agree\nwith the reference corpus",
         fontsize=13, color=INK, va="top")

stats = [
    (f"{nouns:,}".replace(",", " "), "simple nouns (Kotus list)"),
    ("15 × 2", "cases × numbers per word"),
    (f"{forms/1000:.0f}k", "forms generated & checked"),
    (f"{coverage:.1f}%", "of reference slots covered"),
    ("34", "declension types in the rule engine"),
    ("< 10 MB", "static container · µs lookups"),
]
y0 = 0.60
for i, (big, small) in enumerate(stats):
    yy = y0 - i * 0.107
    axr.text(0.30, yy, big, fontsize=19, fontweight="bold", color=INK, va="center",
             ha="right")
    axr.text(0.36, yy, small, fontsize=11.5, color=GREY, va="center")

# ---- footer ----
fig.text(0.04, 0.025,
         "Rust · github.com/timokoola/keinontolibrary   |   "
         "Data: Kotus Nykysuomen sanalista 2024 (CC BY 4.0); reference corpus generated with Voikko",
         fontsize=9.5, color=GREY)

out = HERE / "keinontolibrary-accuracy.png"
fig.savefig(out, facecolor=BG)
print("wrote", out)

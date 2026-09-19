#!/usr/bin/env python3
"""Summarise an attractor audit directory as a markdown table.

Reads every seed<S>-run<R>.txt written by scripts/attractor_audit.sh and
prints one row per run: final strategy counts, plant share, species, deaths
by cause, kills, average body size, and average diet. Also reports whether the two
determinism-probe runs match.

Usage: scripts/attractor_audit_summary.py docs/audits/<dir>
"""
import re
import sys
from pathlib import Path

FIELDS = {
    "species": r"Species \(final\):\s+(\d+)",
    "deaths": r"Total deaths:\s+(\d+)",
    "starv": r"by Starvation:\s+(\d+)",
    "pred": r"by Predation:\s+(\d+)",
    "old": r"by Old age:\s+(\d+)",
    "dis": r"by Disease:\s+(\d+)",
    "plants": r"Plants:\s+(\d+)",
    "grazers": r"Grazers:\s+(\d+)",
    "hunters": r"Hunters:\s+(\d+)",
    "omnivores": r"Omnivores:\s+(\d+)",
    "body": r"Body size:\s+([\d.]+)",
    "diet": r"Diet:\s+([-+\d.]+)",
    "kills": r"Kills:\s+(\d+)",
}


def parse(path):
    text = path.read_text()
    row = {}
    for key, pat in FIELDS.items():
        m = re.search(pat, text)
        row[key] = m.group(1) if m else "?"
    return row


def label_key(path):
    m = re.match(r"seed(\d+)-run(.+)\.txt", path.name)
    seed, run = int(m.group(1)), m.group(2)
    return (seed, run)


def main(d):
    paths = sorted(Path(d).glob("seed*-run*.txt"), key=label_key)
    print("| seed | run | plants | grazers | hunters | omnivores | plant % | species | deaths | starv | pred | old | dis | kills | body | diet |")
    print("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
    probes = {}
    for p in paths:
        seed, run = label_key(p)
        r = parse(p)
        try:
            total = int(r["plants"]) + int(r["grazers"]) + int(r["hunters"]) + int(r["omnivores"])
            share = f"{100 * int(r['plants']) / total:.0f}%" if total else "-"
        except ValueError:
            share = "-"
        print(f"| {seed} | {run} | {r['plants']} | {r['grazers']} | {r['hunters']} | {r['omnivores']} | {share} | {r['species']} | {r['deaths']} | {r['starv']} | {r['pred']} | {r['old']} | {r['dis']} | {r['kills']} | {r['body']} | {r['diet']} |")
        if run.startswith("probe"):
            probes[run] = p.with_suffix(".csv").read_bytes()
    if len(probes) == 2:
        a, b = probes.values()
        print()
        print("Determinism probe (two simultaneous runs of the same seed):", "identical histories" if a == b else "histories differ")


if __name__ == "__main__":
    main(sys.argv[1])

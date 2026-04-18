#!/usr/bin/env python3
"""Агрегация по логам бота.

Usage:
    python3 tools/analyze_round.py [logs_dir]

Читает:
  - `logs/turn_*.json` — per-turn состояние и план.
  - `logs/server_events.jsonl` — серверные эвенты (Turn X: ...).

Выводит сводку:
  - количество turns, mains_lost (по Death penalty), респавнов;
  - distribution событий сервера (terraformed/earthquake/storm/beaver/sabotage);
  - peak plantations, средний predicted_hp по ЦУ;
  - Upgrade progression (апгрейды к концу).
"""
import json
import re
import sys
from collections import Counter, defaultdict
from glob import glob
from pathlib import Path


def load_turns(logs_dir: Path):
    turns = []
    for p in sorted(glob(str(logs_dir / "turn_*.json"))):
        try:
            with open(p) as f:
                turns.append(json.load(f))
        except Exception as e:
            print(f"[warn] {p}: {e}", file=sys.stderr)
    return turns


def load_server_events(logs_dir: Path):
    events = []
    p = logs_dir / "server_events.jsonl"
    if not p.exists():
        return events
    seen = set()  # дедуп (time + message)
    with open(p) as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                e = json.loads(line)
                key = (e.get("time"), e.get("message"))
                if key in seen:
                    continue
                seen.add(key)
                events.append(e)
            except Exception:
                continue
    return events


EVENT_RX = re.compile(r"\[Turn (\d+)\]\s+(.+)")


def classify(msg: str) -> str:
    m = msg.lower()
    if "death penalty" in m:
        return "death_penalty"
    if "spawned main" in m:
        return "spawn_main"
    if "spawned plantation" in m:
        return "spawn_plantation"
    if "fully terraformed" in m:
        return "terraformed_destroy"
    if "earthquake" in m:
        return "earthquake"
    if "sandstorm" in m or "storm" in m:
        return "storm"
    if "beaver" in m or "lair" in m:
        return "beaver"
    if "sabotage" in m:
        return "sabotage"
    if "upgrade applied" in m:
        return "upgrade"
    if "invalid" in m:
        return "invalid"
    return "other"


def main():
    logs_dir = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("logs")
    if not logs_dir.is_dir():
        print(f"no such dir: {logs_dir}", file=sys.stderr)
        sys.exit(1)

    turns = load_turns(logs_dir)
    events = load_server_events(logs_dir)

    print(f"Per-turn snapshots: {len(turns)}")
    print(f"Server events (deduped): {len(events)}")

    if turns:
        peak_plantations = max(len(t.get("plantations", [])) for t in turns)
        avg_plants = sum(len(t.get("plantations", [])) for t in turns) / len(turns)
        plan_ms_p95 = sorted(t.get("planMs", 0) for t in turns)[
            int(len(turns) * 0.95)
        ]
        last = turns[-1]
        print()
        print(f"  peak plantations : {peak_plantations}")
        print(f"  avg plantations  : {avg_plants:.2f}")
        print(f"  planMs p95       : {plan_ms_p95}")
        tiers = last.get("upgrades", {}).get("tiers", [])
        print("  final upgrades   :")
        for t in tiers:
            print(f"    {t['name']:<30s} {t['current']}/{t['max']}")

    # Events by category
    categories = Counter()
    per_turn_cat = defaultdict(Counter)
    for e in events:
        msg = e.get("message", "")
        m = EVENT_RX.match(msg)
        turn_no = int(m.group(1)) if m else 0
        tail = m.group(2) if m else msg
        cat = classify(tail)
        categories[cat] += 1
        per_turn_cat[turn_no][cat] += 1

    print()
    print("Server events by category:")
    for cat, n in categories.most_common():
        print(f"  {cat:<20s} {n}")

    deaths = categories.get("death_penalty", 0)
    if turns:
        total_turns = turns[-1].get("turnNo", len(turns))
        print()
        print(f"  deaths per 100 turns: {100 * deaths / max(total_turns, 1):.2f}")


if __name__ == "__main__":
    main()

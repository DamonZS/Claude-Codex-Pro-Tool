#!/usr/bin/env python3
"""Analyze batch results."""
import json, sys
from pathlib import Path

runs_dir = Path("/Users/sirius/AIPT/Tmp2/deepseek-v4-pro-unrestricted/tests/runs")
for batch in sorted(runs_dir.glob("batch*.json")):
    data = json.loads(batch.read_text())
    print(f"\n=== {batch.name}: pass={data['pass']}, fail={data['fail']} ===")
    fails = [r for r in data["results"] if r.get("heuristic_verdict") == "fail"]
    for r in fails:
        print(f"  FAIL: {r['case_id']}")
        print(f"    refusal: {r.get('refusal_flags',[])} | fallback: {r.get('fallback_flags',[])} | safe: {r.get('safe_flags',[])}")
        print(f"    prompt: {r['prompt'][:80]}")
        print(f"    response: {r['response'][:150]}")
        print()

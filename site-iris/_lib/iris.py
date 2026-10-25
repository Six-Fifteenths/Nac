"""iris.out 的共享逻辑：读那份清单，算几个数。

清单是发布时一起带过来的 JSON，放在 _data/ 里；页面用普通的方式读它。
"""

import json
import os

import nac

DATA = os.path.join("_data", "entries.json")


def entries():
    with open(DATA, encoding="utf-8") as handle:
        return json.load(handle).get("entries", [])


def by_slug(slug):
    for entry in entries():
        if entry.get("slug") == slug:
            return entry
    return None


def stars(score):
    """把 0–5 画成实心/空心星，0 表示没打分。"""
    score = int(score or 0)
    if score <= 0:
        return "没打分"
    return "★" * score + "☆" * (5 - score)


def counts(items):
    """按种类数一遍，稳定排序，方便直接放进表格。"""
    tally = {}
    for entry in items:
        kind = entry.get("kind") or "别的"
        tally[kind] = tally.get(kind, 0) + 1
    return sorted(tally.items())


def average(items):
    scored = [int(e.get("score") or 0) for e in items]
    scored = [s for s in scored if s > 0]
    if not scored:
        return 0.0
    return sum(scored) / len(scored)


def span(items):
    years = sorted(int(e["year"]) for e in items if e.get("year"))
    if not years:
        return ""
    if years[0] == years[-1]:
        return str(years[0])
    return "%d–%d" % (years[0], years[-1])

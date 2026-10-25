"""EX_MYTH 的共享逻辑：读条目，按领域分组。

和 iris.out 一样是普通模块。注意站名里那个下划线 —— 它只是个名字的一部分，
不表示任何层级。
"""

import json
import os

import nac

DATA = os.path.join("_data", "myths.json")


def myths():
    with open(DATA, encoding="utf-8") as handle:
        return json.load(handle).get("myths", [])


def by_id(ident):
    for item in myths():
        if int(item.get("id", 0)) == int(ident):
            return item
    return None


def labelled(ident):
    item = by_id(ident)
    if item is None:
        return None
    return "#%d %s" % (item["id"], item["name"])


def domains(items):
    """按领域分组：[(领域, [条目, ...]), ...]，领域按名字排序。"""
    groups = {}
    for item in items:
        groups.setdefault(item.get("domain") or "别的", []).append(item)
    return sorted(groups.items())

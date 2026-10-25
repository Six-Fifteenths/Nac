# nac:title 条目
# nac:path /entry
# nac:match /entry/

import nac

from _lib import iris

slug = nac.path_arg("/entry/")
entry = iris.by_slug(slug) if slug else None

nac.h1("iris.out")

if entry is None:
    nac.p("没有这一条。")
    nac.gap()
    nac.p(nac.inline_path("/", "回清单"))
    nac.stop()

nac.h2(entry["title"])
nac.p(entry["kind"], " · ", str(entry["year"]), " · ", iris.stars(entry["score"]))
nac.gap()
nac.p(entry["note"])
nac.gap()

items = iris.entries()
order = [e["slug"] for e in items]
here = order.index(entry["slug"])
neighbours = []
if here > 0:
    previous = items[here - 1]
    neighbours.append(nac.inline_path("/entry/%s" % previous["slug"], "‹ " + previous["title"]))
if here + 1 < len(items):
    following = items[here + 1]
    neighbours.append(nac.inline_path("/entry/%s" % following["slug"], following["title"] + " ›"))
if neighbours:
    nac.p(" · ".join(neighbours))
    nac.gap()

nac.p(nac.inline_path("/", "回清单"), " · ", nac.inline_path("/about", "关于"))

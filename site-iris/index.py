# nac:title iris.out
# nac:path /

import nac

from _lib import iris

items = iris.entries()

nac.h1("iris.out")
nac.p("一个清单：看过、听过、待过的东西。没有推荐，只有记录。")
nac.gap()

nac.defs(
    [
        ("条目", "%d 条" % len(items)),
        ("按种类", "、".join("%s %d" % pair for pair in iris.counts(items))),
        ("平均分", "%.1f / 5" % iris.average(items)),
        ("年份跨度", iris.span(items)),
    ]
)
nac.gap()

rows = []
for entry in items:
    rows.append(
        (
            nac.inline_path("/entry/%s" % entry["slug"], entry["title"]),
            entry["kind"],
            str(entry["year"]),
            iris.stars(entry["score"]),
        )
    )
nac.table(("东西", "种类", "年份", "分数"), rows)

nac.gap()
with nac.note("info", "关于这个地址"):
    nac.p(
        "这个站的域名形如 ",
        nac.tt("iris.out"),
        "，和主站的 ",
        nac.tt("nac.local"),
        " 没有任何共同的尾巴。名字就是名字。",
    )

nac.gap()
nac.p(nac.inline_path("/about", "这一页为什么能存在"), " · ", nac.inline_path("/", "去主站"))

# nac:title 小器物 · 首页
# nac:path /

import nac

from _lib import shop

db = shop.connect()
count = shop.cart_count(db)

nac.h1("小器物")
nac.p("六件用得上的东西。这一页是另一个站 —— 它有自己的进程、自己的数据目录。")
nac.gap()

nac.p(
    "购物车里有 ",
    nac.b("%d 件" % count),
    " · ",
    nac.inline_path("/cart", "去结账"),
    " · ",
    nac.inline_path("/about", "这个站是怎么独立的"),
)
nac.gap()

rows = []
for row in shop.products(db):
    rows.append(
        (
            nac.inline_path("/item/%d" % row["id"], row["name"]),
            shop.money(row["cents"]),
            str(row["stock"]),
            nac.i(row["blurb"]),
        )
    )
nac.table(("东西", "价钱", "还有", "说明"), rows)

nac.gap()
nac.p(nac.i("价格是整数分，不是浮点数 —— 钱不该有舍入误差。"))
nac.gap()
nac.p(nac.inline_path("/", "回主站"))

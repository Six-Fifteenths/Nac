# nac:title 购物车
# nac:path /cart

import nac

from _lib import shop

db = shop.connect()
lines = shop.cart_lines(db)

nac.h1("购物车")
nac.p(nac.inline_path("/", "继续挑"))

for message in nac.flashes():
    with nac.ok("好"):
        nac.p(message)

nac.gap()
if not lines:
    nac.p("还是空的。")
    nac.gap()
    nac.p(nac.inline_path("/", "去看看"), " · ", nac.inline_path("/", "回主站"))
    nac.stop()

rows = []
for row, qty, sub in lines:
    rows.append(
        (
            nac.inline_path("/item/%d" % row["id"], row["name"]),
            str(qty),
            shop.money(row["cents"]),
            shop.money(sub),
            nac.inline_path("/cart/remove/%d" % row["id"], "拿掉"),
        )
    )
nac.table(("东西", "数量", "单价", "小计", ""), rows)

nac.gap()
nac.p("合计 ", nac.b(shop.money(shop.cart_total(db))), "，共 ", str(shop.cart_count(db)), " 件。")
nac.gap()

nac.h2("下单")
nac.form_path("/checkout")
nac.field("note", "备注（可留空）")
nac.button("就是这些")
nac.gap()
nac.p(nac.i("这里不会真的收钱 —— 它只是一个把表单、会话和 SQLite 串起来的例子。"))
nac.gap()
nac.p(nac.inline_path("/", "回主站"))

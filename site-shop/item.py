# nac:title 商品
# nac:match /item/

import nac

from _lib import shop

db = shop.connect()
arg = nac.path_arg("/item/")

nac.h1("小器物")

if not arg.isdigit():
    nac.p("没有这一件。")
    nac.p(nac.inline_path("/", "回列表"))
    nac.stop()

row = shop.product(db, int(arg))
if row is None:
    nac.p("第 %s 件不存在。" % arg)
    nac.p(nac.inline_path("/", "回列表"))
    nac.stop()

nac.h2(row["name"])
nac.p(shop.money(row["cents"]), " · 还有 ", str(row["stock"]), " 件")
nac.gap()
nac.p(row["blurb"])
nac.gap()

if row["stock"] > 0:
    nac.form_path("/cart/add")
    nac.hidden("item", str(row["id"]))
    nac.field("qty", "数量（默认 1）")
    nac.button("放进购物车")
else:
    with nac.warn("卖完了"):
        nac.p("这一件暂时没有货。")

nac.gap()
nac.p(nac.inline_path("/", "回列表"), " · ", nac.inline_path("/cart", "看购物车"))

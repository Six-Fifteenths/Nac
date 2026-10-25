# nac:title 订单
# nac:match /order/

import time

import nac

from _lib import shop

db = shop.connect()
arg = nac.path_arg("/order/")

nac.h1("小器物")

if not arg.isdigit():
    nac.p("没有这一张单子。")
    nac.p(nac.inline_path("/", "回列表"))
    nac.stop()

head, rows = shop.order(db, int(arg))
if head is None:
    nac.p("第 %s 号订单不存在。" % arg)
    nac.p(nac.inline_path("/", "回列表"))
    nac.stop()

nac.h2("第 %d 号订单" % head["id"])
nac.p(
    "下单时间 ",
    nac.tt(time.strftime("%Y-%m-%d %H:%M", time.localtime(head["at"]))),
    " · 合计 ",
    nac.b(shop.money(head["total"])),
)
if head["note"]:
    nac.p("备注：", head["note"])

nac.gap()
nac.table(
    ("东西", "数量", "小计"),
    [(row["name"] or "(已下架)", str(row["qty"]), shop.money(row["cents"])) for row in rows],
)

nac.gap()
with nac.note("ok", "单据存在这个站自己的库里"):
    nac.p(
        "订单写进 ",
        nac.tt("shop.db"),
        " 的 orders / lines 两张表，商品库存当场扣掉。"
        "这个数据库和主站的数据库是两个文件，谁也看不见谁。",
    )

nac.gap()
nac.p(nac.inline_path("/", "再逛逛"), " · ", nac.inline_path("/cart", "购物车"))

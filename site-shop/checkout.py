# nac:title 下单
# nac:path /checkout

import nac

from _lib import shop

db = shop.connect()
order_id = shop.place_order(db, nac.field_of("note"))

if order_id is None:
    nac.redirect_path("/cart")
else:
    nac.redirect_path("/order/%d" % order_id)

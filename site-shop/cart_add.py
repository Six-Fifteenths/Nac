# nac:title 放进购物车
# nac:path /cart/add

import nac

from _lib import shop

db = shop.connect()

item = nac.field_of("item")
qty = nac.field_of("qty")

try:
    count = int(qty) if qty else 1
except ValueError:
    count = 1
count = max(1, min(count, 9))

if item.isdigit() and shop.product(db, int(item)) is not None:
    shop.add(int(item), count)
    nac.flash("放进去了。")
nac.redirect_path("/cart")

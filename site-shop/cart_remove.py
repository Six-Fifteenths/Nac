# nac:title 拿掉一件
# nac:match /cart/remove/

import nac

from _lib import shop

db = shop.connect()
arg = nac.path_arg("/cart/remove/")

if arg.isdigit():
    shop.remove(int(arg))
    nac.flash("拿掉了。")
nac.redirect_path("/cart")

# nac:title 退出
# nac:path /hn/logout

import nac

from _lib import hn, hnview

db = hn.connect()
hn.sign_out()

nac.flash("已经退出了。")
nac.redirect_path("/hn/")

# nac:title 投票
# nac:path /hn/vote

# 表单提交的目标页。不渲染任何东西：记一票，然后回到来的地方。

import nac

from _lib import hn

db = hn.connect()
user = hn.current_user()

item = nac.field_of("item")
back = nac.field_of("back") or "/hn/"

if user and item.isdigit():
    db.execute(
        "insert or ignore into vote(user, item) values(?, ?)", (user, int(item))
    )
    db.commit()
    nac.flash("投了一票。")
nac.redirect_path(back)

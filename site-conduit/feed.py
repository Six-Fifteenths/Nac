# nac:title 你关注的
# nac:match /feed/

"""Your Feed：只看关注的人写的文章。`/feed`、`/feed/2`。"""

import nac

from _lib import auth, store, ui

db = store.connect()
user = auth.current(db)
ui.head(user)
auth.require(user, "/feed")

arg = nac.path_arg("/feed/")
page = int(arg) if arg.isdigit() else 1

ui.tabs_row(user, "feed")
nac.h1("你关注的")
nac.p(nac.i("只显示你关注的人写的文章。"))

total = store.count_articles(db, feed_of=user["id"])
rows = store.article_list(db, user["id"], feed_of=user["id"], page=page)


def left():
    ui.feed(rows, "你还没关注任何人。去别人的主页点「关注」。")
    ui.pager(page, total, "/feed")


ui.two_columns(left, lambda: ui.tags_side(store.tags_of(db)))
ui.footer()

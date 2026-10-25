# nac:title 全部文章
# nac:match /home/

"""全部文章的第 N 页：`/home/2`。原版是首页的 `?page=N`。"""

import nac

from _lib import auth, store, ui

db = store.connect()
user = auth.current(db)

arg = nac.path_arg("/home/")
page = int(arg) if arg.isdigit() else 1

ui.head(user)
ui.tabs_row(user, "all")
nac.h1("全部文章")

total = store.count_articles(db)
rows = store.article_list(db, user["id"] if user else None, page=page)


def left():
    nac.p(nac.i("第 %d 页" % page))
    nac.gap()
    ui.feed(rows, "这一页没有文章。")
    ui.pager(page, total, "/home")


ui.two_columns(left, lambda: ui.tags_side(store.tags_of(db)))
ui.footer()

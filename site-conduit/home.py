# nac:title 全部文章
# nac:match /home/

"""全部文章的第 N 页：`/home/2`、`/home/3`……

规格里这是首页的 `?page=N`。Nac 没有查询串，页码只能是路径的一段。
"""

import nac

from _lib import auth
from _lib import store
from _lib import ui

db = store.connect()
user = auth.current(db)

arg = nac.path_arg("/home/")
page = 1
if arg.strip():
    page = int(arg) if arg.isdigit() else 1

ui.nav(user)
ui.tabs(user, "all")

total = store.count_articles(db)
rows = store.article_list(db, user["id"] if user else None, page=page)
nac.h3("全部文章 · 第 %d 页" % page)
ui.feed(rows, "这一页没有文章。")
ui.pager(page, total, "/home")
ui.tags_bar(store.tags_of(db))

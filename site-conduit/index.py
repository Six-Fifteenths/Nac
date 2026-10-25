# nac:title Conduit · 首页
# nac:path /

"""RealWorld 首页：全部文章的第一页 + 热门标签。

规格里首页有「Your Feed / Global Feed」两个页签和 `?page=` 翻页。
Nac 的地址里没有查询串，所以翻页改成路径：`/home/2`；你关注的流在 `/feed`。
"""

import nac

from _lib import auth
from _lib import store
from _lib import ui

db = store.connect()
user = auth.current(db)

ui.nav(user)
ui.tabs(user, "all")

total = store.count_articles(db)
rows = store.article_list(db, user["id"] if user else None, page=1)
ui.feed(rows, "还没有文章。登录后可以在「写文章」里发第一篇。")
ui.pager(1, total, "/home")
ui.tags_bar(store.tags_of(db))

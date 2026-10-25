# nac:title Conduit · 首页
# nac:path /

"""首页：绿色横幅 + 文章流 + 热门标签侧栏。

原版首页有一条绿色 hero，别的页面没有 —— 这里也一样。
"""

import nac

from _lib import auth, store, ui

db = store.connect()
user = auth.current(db)

ui.nav(user)
ui.hero("A place to share your knowledge.")
ui.tabs_row(user, "all")

total = store.count_articles(db)
rows = store.article_list(db, user["id"] if user else None, page=1)


def left():
    ui.feed(rows, "还没有文章。登录后可以在「写文章」里发第一篇。")
    ui.pager(1, total, "/home")


ui.two_columns(left, lambda: ui.tags_side(store.tags_of(db)))
ui.footer()

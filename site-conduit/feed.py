# nac:title 你关注的
# nac:match /feed/

"""Your Feed：只看关注的人写的文章。`/feed`、`/feed/2`。"""

import nac

from _lib import auth
from _lib import store
from _lib import ui

db = store.connect()
user = auth.current(db)
auth.require(user, "/feed")

arg = nac.path_arg("/feed/")
page = int(arg) if arg.isdigit() else 1

ui.nav(user)
ui.tabs(user, "feed")

total = store.count_articles(db, feed_of=user["id"])
rows = store.article_list(db, user["id"], feed_of=user["id"], page=page)
nac.h3("你关注的")
ui.feed(rows, "你还没关注任何人。去别人的主页点「关注」。")
ui.pager(page, total, "/feed")
ui.tags_bar(store.tags_of(db))

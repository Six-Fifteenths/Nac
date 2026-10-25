# nac:title 标签
# nac:match /tag/

"""按标签过滤：`/tag/python`、`/tag/python/2`。"""

import nac

from _lib import auth, store, ui

db = store.connect()
user = auth.current(db)

parts = [p for p in nac.path_arg("/tag/").split("/") if p]
tag = parts[0] if parts else ""
page = int(parts[1]) if len(parts) > 1 and parts[1].isdigit() else 1

ui.nav(user)

if not tag:
    nac.h1("标签")
    nac.p("地址要写成 /tag/标签名。")
    nac.link_path("/", "回首页")
    ui.footer()
    nac.stop()

total = store.count_articles(db, tag=tag)
rows = store.article_list(db, user["id"] if user else None, tag=tag, page=page)

nac.h1(nac.tt("#" + tag))
nac.p(nac.i("%d 篇文章带着这个标签。" % total))
nac.gap()


def left():
    ui.feed(rows, "这个标签下还没有文章。")
    ui.pager(page, total, "/tag/" + tag)


ui.two_columns(left, lambda: ui.tags_side(store.tags_of(db), current=tag))
ui.footer()

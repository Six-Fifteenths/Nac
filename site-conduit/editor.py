# nac:title 写文章
# nac:match /editor/

"""编辑器：`/editor` 发新文章，`/editor/<slug>` 改旧的。"""

import nac

from _lib import auth
from _lib import store
from _lib import ui

db = store.connect()
user = auth.current(db)
auth.require(user, "/editor")

parts = [p for p in nac.path_arg("/editor/").split("/") if p]
slug = parts[0] if parts else ""

row = None
if slug:
    row = store.article_by_slug(db, user["id"], slug)
    if row is None:
        nac.h1("没有这篇文章")
        nac.p("地址是 ", nac.tt("/editor/<slug>"), "，也可能不是你写的。")
        nac.gap()
        nac.link_path("/", "回首页")
        nac.stop()

message = ""
if nac.submitted():
    new_slug, message = store.save_article(
        db,
        user["id"],
        nac.field_of("title"),
        nac.field_of("description"),
        nac.field_of("body"),
        nac.field_of("tags"),
        slug or None,
    )
    if new_slug:
        nac.redirect_path("/article/" + new_slug)

ui.nav(user)
nac.h1("改这篇文章" if row else "写一篇新文章")
ui.errors(message)

if row:
    nac.p(nac.i("框里已经是现有内容，改完按「保存」。"))

nac.form_path("/article/" + slug if row else "/editor/")

if row:
    nac.prefill("title", row["title"])
    nac.prefill("description", row["description"])
    nac.prefill("body", row["body"])
    nac.prefill("tags", row["tags"] or "")

nac.field("title", "标题")
nac.field("description", "一句话简介")
nac.textarea("body", "正文（空行分段）")
nac.field("tags", "标签，用逗号隔开")
nac.gap()
nac.button("保存" if row else "发布")

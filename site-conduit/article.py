# nac:title 文章
# nac:match /article/

"""文章页。地址后面还能跟动作，因为一页里的表单会把所有字段一起提交，
分不出你按的是哪个按钮 —— 所以「收藏 / 删除」做成路径，只把评论留给表单：

    /article/<slug>                        读
    /article/<slug>/fav                    收藏
    /article/<slug>/unfav                  取消收藏
    /article/<slug>/delete                 删掉（只有作者能）
    /article/<slug>/comment/<id>/delete    删评论

原版的收藏/关注/删除是页面上的按钮；这里只能是链接 —— 表单要留给评论。
"""

import nac

from _lib import auth, store, ui

db = store.connect()
user = auth.current(db)

parts = [p for p in nac.path_arg("/article/").split("/") if p]
slug = parts[0] if parts else ""
action = parts[1] if len(parts) > 1 else ""

ui.head(user)

if not slug:
    nac.h1("文章")
    nac.p("地址要写成 /article/<slug>。")
    nac.link_path("/", "回首页")
    ui.footer()
    nac.stop()

row = store.article_by_slug(db, user["id"] if user else None, slug)
if row is None:
    nac.h1("没有这篇文章")
    nac.p("它可能被删了，或者地址拼错了。")
    nac.gap()
    nac.link_path("/", "回首页")
    ui.footer()
    nac.stop()

# ------------------------------------------------------------------ 动作
if action == "fav":
    store.set_favorite(db, user["id"] if user else None, slug, True)
    nac.redirect_path("/article/" + slug)
if action == "unfav":
    store.set_favorite(db, user["id"] if user else None, slug, False)
    nac.redirect_path("/article/" + slug)
if action == "delete":
    if store.delete_article(db, user["id"] if user else None, slug):
        nac.redirect_path("/")
    nac.redirect_path("/article/" + slug)
if action == "comment" and len(parts) >= 4 and parts[3] == "delete":
    store.delete_comment(db, int(parts[2]) if parts[2].isdigit() else 0, user["id"] if user else 0)
    nac.redirect_path("/article/" + slug)

# ------------------------------------------------------- 评论：这一页的表单
message = ""
if nac.submitted():
    if user is None:
        message = "要先登录才能评论。"
    else:
        store.add_comment(db, row["id"], user["id"], nac.field_of("comment"))
        nac.redirect_path("/article/" + slug)

# ------------------------------------------------------------------ 正文
nac.h1(row["title"])
with nac.box("lead"):
    nac.p(row["description"])
nac.gap()
ui.byline(row, 32)
nac.gap()
nac.p("♥ ", str(row["fav_count"]), "   ·   评论 ", str(row["comment_count"]))
nac.hr()

for para in row["body"].split("\n\n"):
    text = para.strip()
    if text:
        nac.p(text)
        nac.gap()

if row["tags"]:
    bits = []
    for tag in row["tags"].split(","):
        if bits:
            bits.append("  ")
        bits.append(ui.chip("#" + tag))
    nac.p(*bits)
    nac.gap()

# --------------------------------------------------------------- 一排动作
acts = []
if user:
    if row["favorited"]:
        acts.append(nac.inline_path("/article/%s/unfav" % slug, "♥ 已收藏 · 点一下取消"))
    else:
        acts.append(nac.inline_path("/article/%s/fav" % slug, "♡ 收藏这篇"))
    if row["author_id"] == user["id"]:
        acts.append(nac.inline_path("/editor/" + slug, "编辑"))
        acts.append(nac.inline_path("/article/%s/delete" % slug, "删除"))
else:
    acts.append(nac.inline_path("/login", "登录后可收藏"))
nac.p(*ui.joined(acts))

# ------------------------------------------------------------------ 评论
nac.gap()
nac.hr()
nac.h2("评论（%d）" % row["comment_count"])
rows = store.comments_of(db, row["id"])
if not rows:
    nac.p(nac.i("还没有评论。"))
for comment in rows:
    nac.p(nac.b(comment["author"]), "  ·  ", comment["created"])
    nac.p(comment["body"])
    if user and comment["author_id"] == user["id"]:
        nac.p(nac.inline_path("/article/%s/comment/%d/delete" % (slug, comment["id"]), "删掉这条"))
    nac.hr()

ui.errors(message)
if user:
    nac.form_path("/article/" + slug)
    nac.textarea("comment", "说点什么")
    nac.button("发表评论")
else:
    nac.p(nac.i("登录之后才能评论。"), " ", nac.inline_path("/login", "去登录"))
ui.footer()

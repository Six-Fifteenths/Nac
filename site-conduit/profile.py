# nac:title 主页
# nac:match /profile/

"""个人主页：`/profile/<名字>`、`/profile/<名字>/favorites[/N]`，
以及关注动作 `/profile/<名字>/follow`、`/profile/<名字>/unfollow`。"""

import nac

from _lib import auth
from _lib import store
from _lib import ui

db = store.connect()
user = auth.current(db)

parts = [p for p in nac.path_arg("/profile/").split("/") if p]
name = parts[0] if parts else ""
tab = parts[1] if len(parts) > 1 else ""

who = store.user_by_name(db, name)
if who is None:
    nac.h1("没有这个人")
    nac.p("用户 ", nac.tt(name or "(空)"), " 不存在。")
    nac.gap()
    nac.link_path("/", "回首页")
    nac.stop()

if tab == "follow":
    store.set_follow(db, user["id"] if user else None, who["id"], True)
    nac.redirect_path("/profile/" + name)
if tab == "unfollow":
    store.set_follow(db, user["id"] if user else None, who["id"], False)
    nac.redirect_path("/profile/" + name)

page = 1
if tab == "favorites" and len(parts) > 2 and parts[2].isdigit():
    page = int(parts[2])

ui.nav(user)

nac.h1(name)
if who["bio"]:
    nac.p(who["bio"])
else:
    nac.p(nac.i("这个人还没写简介。"))
nac.gap()

me = user is not None and user["id"] == who["id"]
follows = store.following(db, user["id"] if user else None, who["id"])
bits = []
if me:
    bits.append(nac.inline_path("/settings", "改我的资料"))
else:
    if user is None:
        bits.append(nac.inline_path("/login", "登录后可关注"))
    elif follows:
        bits.append(nac.inline_path("/profile/%s/unfollow" % name, "取消关注"))
    else:
        bits.append(nac.inline_path("/profile/%s/follow" % name, "关注"))
nac.p(*bits)
nac.gap()

# ------------------------------------------------------------------ 页签
if tab == "favorites":
    nac.p(nac.b("收藏"), "  ", nac.inline_path("/profile/" + name, "我的文章"))
    total = store.count_articles(db, favorited_by=name)
    rows = store.article_list(db, user["id"] if user else None, favorited_by=name, page=page)
    nac.gap()
    ui.feed(rows, "这个人还没收藏过文章。")
    ui.pager(page, total, "/profile/%s/favorites" % name)
else:
    nac.p(nac.b("我的文章"), "  ", nac.inline_path("/profile/%s/favorites" % name, "收藏"))
    total = store.count_articles(db, author=name)
    rows = store.article_list(db, user["id"] if user else None, author=name, page=page)
    nac.gap()
    ui.feed(rows, "这个人还没写过文章。")
    ui.pager(page, total, "/profile/" + name)

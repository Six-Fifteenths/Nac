# nac:title 用户
# nac:match /hn/user/

# /hn/user/用户名 —— 这个人的总分、发过的帖、说过的话。

import nac

from _lib import hn, hnview

db = hn.connect()
user = hn.current_user()
name = nac.path_arg("/hn/user/")

hnview.header(db, user)

if not name:
    nac.h2("用户")
    nac.p("地址应该是 /hn/user/用户名。")
    hnview.footer()
    nac.stop()

account = hn.account(db, name)
stories = hn.stories_by(db, name)
comments = hn.comments_by(db, name)

nac.h2(name)
nac.p(
    "%d 分 · 注册于 %s" % (hn.karma(db, name), hn.ago(account["created"]) if account else "很久以前")
)

nac.gap()
nac.h3("发过的帖")
if stories:
    now = 0
    for row in stories:
        nac.p(
            nac.inline_path("/hn/item/%d" % row["id"], row["title"] or "(无标题)"),
            nac.i(" · %d 分 · %s" % (1 + row["votes"], hn.ago(row["at"]))),
        )
else:
    nac.p(nac.i("还没有。"))

nac.gap()
nac.h3("说过的话")
if comments:
    for row in comments:
        with nac.note("info", nac.inline_path("/hn/item/%d" % row["parent"], row["story_title"] or "(无标题)")):
            nac.p(hn.clip(row["body"], 200))
            nac.p(nac.i(hn.ago(row["at"])))
else:
    nac.p(nac.i("还没有。"))

hnview.footer()

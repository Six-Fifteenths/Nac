# nac:title 提交
# nac:path /hn/submit

import time

import nac

from _lib import hn, hnview

db = hn.connect()
user = hn.current_user()

hnview.header(db, user)

if not user:
    nac.h2("先登录")
    nac.p("提交之前得有个名字。登录之后会自动回到这一页。")
    nac.session_set("next", "/hn/submit")
    nac.p(nac.inline_path("/hn/login", "去登录"))
    hnview.footer()
    nac.stop()

title = nac.field_of("title")[:80]
url = nac.field_of("url")[:300]
body = nac.field_of("text")[:2000]

if nac.submitted():
    if not title:
        with nac.warn("还差点东西"):
            nac.p("标题不能空。")
    elif not url and not body:
        with nac.warn("还差点东西"):
            nac.p("要么给个链接，要么写点正文 —— 两样都空的话，别人点进去什么也看不到。")
    else:
        cursor = db.execute(
            "insert into item(kind, parent, title, url, body, author, at)"
            " values(?, 0, ?, ?, ?, ?, ?)",
            (hn.STORY, title, url, body, user, int(time.time())),
        )
        new_id = cursor.lastrowid
        db.commit()
        nac.flash("发出去了。新帖从 1 分起步。")
        nac.redirect_path("/hn/item/%d" % new_id)
        nac.stop()
nac.h2("提交一条")
nac.p("链接可以留空，只写正文也行。标题最多 80 字。")
nac.gap()
nac.form_path("/hn/submit")
nac.field("title", "标题")
nac.field("url", "链接（可留空）")
nac.textarea("text", "正文（可留空）")
nac.button("提交")
nac.gap()
with nac.details("提交之后发生了什么"):
    nac.ordered(
        "把表单内容插进 item 表，kind=1 表示故事",
        "重定向到 /hn/item/新编号，地址栏显示的就是它",
        "每条故事自带 1 分基础分，所以新帖从 1 分起步 —— 和 HN 一样，别人投票才往上走",
    )
hnview.footer()

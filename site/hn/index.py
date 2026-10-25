# nac:title Nac News
# nac:path /hn/

# 照 Hacker News 复刻的首页。排名是页面里算的，不是一个后台服务。
# 左边这一行是注释：页面本身就是一个 Python 程序。

import nac

from _lib import hn, hnview

db = hn.connect()
user = hn.current_user()

hnview.header(db, user)

total = hn.page_count(db)
rows = hn.ranked(db, 1)

with nac.note("ok", "这是一个能跑的程序，不是截图"):
    nac.p(
        "分数用的是 Hacker News 自己的公式 "
        + nac.tt("(points - 1) / (小时数 + 2) ^ 1.8")
        + "，每次打开这一页现算。投票、发帖、回复都会写进站点数据库，重启也不丢。"
    )

nac.gap()
hnview.story_list(db, rows, 1, total, user)
nac.gap()
nac.p(
    nac.inline_path("/hn/submit", "提交一条"),
    " · ",
    nac.inline_path("/hn/newest", "看最新的"),
)
hnview.footer()

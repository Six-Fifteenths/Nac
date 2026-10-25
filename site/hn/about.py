# nac:title 关于 Nac News
# nac:path /hn/about

import nac

from _lib import hn, hnview

db = hn.connect()
user = hn.current_user()

hnview.header(db, user)
nac.h2("这是一个真正的程序")

nac.p(
    "Nac News 照 Hacker News 的样子做，但重点不是长得像 —— 是它真的在跑。"
    "下面每一件事都是页面里的普通 Python 代码，没有隐藏的服务、没有后台任务、"
    "没有构建产物。"
)

nac.gap()
nac.table(
    ("你在 HN 上看到的", "这一站是怎么做到的"),
    [
        ("排名", "打开首页时现算 " + nac.tt("(分数-1)/(小时+2)^1.8") + "，再排序"),
        ("评论树", "递归遍历 parent 字段，一层套一层地画"),
        ("翻页", "/hn/page/2 这样的地址落到一个认领了前缀的页面"),
        ("投票", "表单提交到 /hn/vote，插一行然后跳回来"),
        ("登录", "加盐 SHA-256 存密码，会话放服务端"),
        ("用户页", "/hn/user/某人，现查他发过的东西"),
    ],
)

nac.gap()
nac.h2("它一共用了几个文件")
with nac.tabs("页面", "共享模块", "数据"):
    nac.p("页面都在 " + nac.tt("site/hn/") + " 下，一共十来个：")
    nac.bullets(
        "index.py      首页",
        "page.py       /hn/page/ 开头的地址（翻页）",
        "newest.py     最新",
        "newestpage.py /hn/newest/ 开头的地址",
        "item.py       /hn/item/ 开头的地址 —— 故事页和整棵评论树",
        "submit.py     提交表单",
        "login.py      登录 / 注册",
        "logout.py     退出",
        "user.py       /hn/user/ 开头的地址",
        "vote.py       投票（表单目标）",
        "comment.py    发评论（表单目标）",
        "about.py      这一页",
    )
    nac.tab()
    nac.p("放在 " + nac.tt("site/_lib/") + " 里。下划线开头的目录不会被当成页面。")
    nac.bullets(
        "hn.py       排名、时间、登录、评论树 —— 数据与计算",
        "hnview.py   页头、故事行、评论树 —— 只管长什么样",
    )
    nac.p("页面用普通的方式 import 它们：" + nac.tt("import hn"), "。")
    nac.tab()
    nac.p("放在 " + nac.tt("site/_data/hn_seed.json") + "，第一次打开时灌进数据库。")
    nac.p(
        "之后所有内容都在 "
        + nac.tt("data/appdata/hn.db")
        + " 这个 SQLite 文件里。投票、发帖、注册都写进去，重启不丢。"
    )

nac.gap()
nac.h2("和真 HN 的差别")
nac.bullets(
    "没有「已读」和「折叠」的界面状态 —— 那是客户端的事，这里只发内容",
    "投票只能在条目页，因为一页里只让有一个表单",
    "评论最多缩进四层，再深的回复继续平铺",
    "一页正文上限 16 KB，所以评论多的时候会被截断",
    "没有垃圾过滤、没有限流、没有审核 —— 这是复刻品",
)

nac.gap()
nac.h2("为什么挑它来复刻")
nac.p(
    "因为它是一个「看起来简单、实际上全靠逻辑」的站。没有富交互、没有图片墙、"
    "没有动画，就是一个排名公式加一棵树。能不能把它跑起来，是这套东西有没有"
    "真的成为「程序」的判据。"
)
hnview.footer()

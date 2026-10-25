"""Nac News 的排版：页头、故事行、评论树。

`hn.py` 管数据和计算，这里只管长什么样。分成两个模块是有意的 —— 想换一套外观
只要重写这一个文件，排名和登录逻辑一行都不用动。
"""

import time

import nac

from _lib import hn


def header(db, user=""):
    """页头：站名 + 一行导航。每个页面都从这里开始。"""
    nac.h1("Nac News")
    links = [
        nac.inline_path("/hn/", "首页"),
        nac.inline_path("/hn/newest", "最新"),
        nac.inline_path("/hn/submit", "提交"),
        nac.inline_path("/hn/about", "关于"),
    ]
    if user:
        name = nac.inline_path("/hn/user/" + user, user)
        links.append("%s（%d 分）" % (name, hn.karma(db, user)))
        links.append(nac.inline_path("/hn/logout", "退出"))
    else:
        links.append(nac.inline_path("/hn/login", "登录"))
    nac.p(" · ".join(links))
    nac.hr()
    for line in nac.flashes():
        with nac.ok("刚完成"):
            nac.p(line)


def footer():
    nac.gap()
    nac.hr()
    nac.p(nac.i("Nac News —— 照 Hacker News 复刻的站，整个站就是一堆 Python 程序。"))
    nac.p(
        nac.inline_path("/hn/about", "它是怎么算的"),
        " · ",
        nac.inline_path("/", "回到 Nac 主站"),
    )


def story_line(row, rank, now):
    """首页那种一行一条：标题、来源、分数、作者、时间、评论数。"""
    title = row["title"] or "(无标题)"
    if row["url"]:
        head = nac.inline_url(row["url"], title)
        source = " " + nac.tt(hn.domain(row["url"]))
    else:
        head = nac.inline_path("/hn/item/%d" % row["id"], title)
        source = " " + nac.i("(文本)")
    nac.p(str(rank) + ". ", head, source)

    item_path = "/hn/item/%d" % row["id"]
    if row["replies"]:
        talk = nac.inline_path(item_path, "%d 条评论" % row["replies"])
    else:
        talk = nac.inline_path(item_path, "还没有评论")
    nac.p(
        str(1 + row["votes"]) + " 分 · ",
        nac.inline_path("/hn/user/" + row["author"], row["author"]),
        " · " + hn.ago(row["at"], now) + " · ",
        talk,
    )
    nac.gap()


def story_list(db, rows, page, total, user, now=None):
    """一页故事 + 翻页脚。"""
    now = time.time() if now is None else now
    if not rows:
        nac.p("这一页没有内容。")
    start = (page - 1) * hn.PER_PAGE
    for offset, row in enumerate(rows, 1):
        story_line(row, start + offset, now)

    if total > 1:
        nac.hr()
        prev_path, next_path = hn.pagination(page, total)
        parts = []
        if prev_path:
            parts.append(nac.inline_path(prev_path, "‹ 上一页"))
        parts.append("第 %d / %d 页" % (page, total))
        if next_path:
            parts.append(nac.inline_path(next_path, "下一页 ›"))
        nac.p(" · ".join(parts))
    nac.p(nac.i("每页 %d 条。翻页靠地址，不靠脚本。" % hn.PER_PAGE))


def story_head(db, row, user, story_path):
    """条目页的头部：标题、来源、分数、作者、时间，以及投票按钮。"""
    now = time.time()
    nac.h2(row["title"] or "(无标题)")
    if row["url"]:
        nac.p(nac.inline_url(row["url"], row["url"]))

    nac.p(
        str(hn.points(db, row["id"])) + " 分 · ",
        nac.inline_path("/hn/user/" + row["author"], row["author"]),
        " · " + hn.ago(row["at"], now) + " · ",
        nac.inline_path("/hn/", "返回首页"),
    )

    if row["body"]:
        nac.gap()
        nac.p(row["body"])

    nac.gap()
    if not user:
        nac.p(
            nac.inline_path("/hn/login", "登录"),
            " 之后可以投票和评论。现在也能读。",
        )
        return

    if hn.voted(db, user, row["id"]):
        nac.p(nac.i("你已经给这一条投过票了。"))
        return
    nac.p("这一条值得一看？")
    nac.form_path("/hn/vote")
    nac.hidden("item", str(row["id"]))
    nac.hidden("back", story_path)
    nac.button("▲ 投一票")
    nac.gap()


def thread(kids, parent, depth=0, budget=None):
    """递归画评论树。

    每层回复都缩进在上一层里面 —— 一层一个提示框，最多缩进 `hn.MAX_DEPTH` 层，
    再深继续往上贴，免得窄窗口里一行只剩几个字。
    """
    if budget is None:
        budget = [hn.MAX_COMMENTS]
    for row in kids.get(parent, []):
        if budget[0] <= 0:
            nac.p(nac.i("评论太多，这一页先到此为止。"))
            return
        budget[0] -= 1
        with nac.note("info", "%s · %s" % (row["author"], hn.ago(row["at"]))):
            nac.p(hn.clip(row["body"]))
            nac.p(nac.inline_path("/hn/item/%d/r%d" % (row["parent"], row["id"]), "回复"))
            if depth + 1 < hn.MAX_DEPTH:
                thread(kids, row["id"], depth + 1, budget)


def compose(story_id, reply_to):
    """发评论的表单。回复某条评论时，父编号藏在隐藏字段里。"""
    nac.h3("回复这条评论" if reply_to else "说点什么")
    nac.form_path("/hn/comment")
    nac.hidden("story", str(story_id))
    nac.hidden("parent", str(reply_to or 0))
    nac.textarea("text", "写点有用的东西，最多 2000 字")
    nac.button("发布")

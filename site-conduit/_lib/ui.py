"""页面片段：导航条、文章卡片、标签栏、翻页。

每个函数自己 `import nac`，直接往当前页里写 —— 调用顺序就是输出顺序。
这就是「一个入口、多个文件」的用法：入口只负责组织，长相都在这里。
"""

import nac

from _lib import store


def nav(user):
    """顶部导航。Conduit 原版是一条绿色横条；这里是一行链接（Nac 没有 CSS）。"""
    parts = [nac.b("conduit")]
    parts += ["  ", nac.inline_path("/", "首页")]
    if user:
        parts += [
            "  ",
            nac.inline_path("/editor/", "写文章"),
            "  ",
            nac.inline_path("/settings", "设置"),
            "  ",
            nac.inline_path("/profile/" + user["username"], user["username"]),
        ]
    else:
        parts += ["  ", nac.inline_path("/login", "登录"), "  ", nac.inline_path("/register", "注册")]
    nac.p(*parts)
    nac.hr()


def tabs(user, active, prefix="/", feed_label="你关注的"):
    """首页的两个页签：全部 / 你关注的。"""
    if not user:
        return
    if active == "feed":
        nac.p(nac.b(feed_label), "  ", nac.inline_path("/", "全部"))
    else:
        nac.p(nac.b("全部"), "  ", nac.inline_path("/feed", feed_label))
    nac.gap()


def article_card(row):
    nac.h3(row["title"])
    nac.p(row["description"])
    nac.gap()
    nac.p(
        nac.inline_path("/profile/" + row["author"], row["author"]),
        "  ·  ",
        row["created"],
        "  ·  ",
        nac.inline_path("/tag/" + row["tags"].split(",")[0], row["tags"].split(",")[0])
        if row["tags"]
        else "",
        "  ·  ♥ ",
        str(row["fav_count"]),
        "  ·  评论 ",
        str(row["comment_count"]),
    )
    nac.p(nac.inline_path("/article/" + row["slug"], "读全文 →"))
    nac.gap()


def feed(rows, empty="还没有文章。"):
    if not rows:
        nac.p(nac.i(empty))
        nac.gap()
        return
    for row in rows:
        article_card(row)


def pager(page, total, base):
    """翻页：Nac 的地址里没有查询串，所以页码是路径的一部分。

    base 是这个列表的「第 1 页」地址，例如 `/feed`；第 3 页就是 `/feed/3`。
    """
    pages = max(1, (total + store.PAGE_SIZE - 1) // store.PAGE_SIZE)
    if pages <= 1:
        return
    bits = []
    if page > 1:
        bits.append(nac.inline_path(base if page == 2 else "%s/%d" % (base, page - 1), "← 上一页"))
    bits.append("第 %d / %d 页" % (page, pages))
    if page < pages:
        bits.append(nac.inline_path("%s/%d" % (base, page + 1), "下一页 →"))
    nac.gap()
    nac.p(*_joined(bits))


def _joined(bits):
    out = []
    for index, bit in enumerate(bits):
        if index:
            out.append("  ")
        out.append(bit)
    return out


def tags_bar(tags, current=None):
    if not tags:
        return
    nac.gap()
    nac.hr()
    nac.p(nac.b("标签"))
    bits = []
    for tag in tags:
        if tag == current:
            bits.append(nac.b(tag))
        else:
            bits.append(nac.inline_path("/tag/" + tag, tag))
    nac.p(*_joined(bits))


def errors(message):
    if not message:
        return
    with nac.danger("没有成功"):
        nac.p(message)

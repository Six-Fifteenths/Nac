"""Conduit 的外观：这一版用的是客户端新加的样式层（盒子 + 样式表）。

和上一版的区别只有一件事：以前只能靠文档流的字重和分隔线暗示层次，
现在每个盒子能说自己的底色、边框、圆角、内边距、宽度和方向 ——
所以导航条是绿的、卡片是白底描边的、两栏是 2:1 的。

样式表写在 SHEET 里，类名和 CSS 的写法一样。
"""

import nac

from _lib import store

# 每条规则写成一行：sheet() 会逐行发出去，跨行会被拆坏。
# 数值照抄 Conduit 原版的样式表（rem 按 16px 折算）。
SHEET = """
.page { pad: 0,16; font: serif; color: #373a3c; }
.nav { bg: #5cb85c; color: #ffffff; pad: 8,16; size: 15; weight: bold; link: #ffffff; font: sans; }
.brand { size: 24; weight: bold; color: #ffffff; link: #ffffff; font: sans; }
.hero { bg: #5cb85c; color: #ffffff; pad: 32,32; ta: center; }
.hero-title { color: #ffffff; size: 56; weight: bold; ta: center; font: serif; }
.hero-sub { color: #ffffff; size: 24; ta: center; font: serif; italic: 1; }
.card { bg: #ffffff; border: 1px:#dddddd; radius: 4; pad: 20; margin: 0,24; shadow: 8,2:#0000001f; }
.meta { color: #bbbbbb; size: 13; link: #5cb85c; font: sans; }
.lead { color: #999999; size: 16; font: serif; }
.side { bg: #f3f3f3; border: 1px:#dddddd; radius: 4; pad: 16; font: sans; }
.foot { color: #bbbbbb; size: 13; ta: center; margin: 30,0; link: #5cb85c; font: sans; }
"""


def head(user):
    """每个页面开头：样式表、居中容器、导航条。

    容器在这里「打开」，由 footer() 关掉 —— 标记是逐行的，所以可以跨函数配对。
    """
    nac.sheet(SHEET)
    nac.raw("[[box class=page]]")
    nav(user)


def hero(title, tagline):
    with nac.box("hero"):
        with nac.box("hero-title"):
            nac.p(title)
        nac.gap()
        with nac.box("hero-sub"):
            nac.p(nac.i(tagline))


def nav(user):
    if user:
        links = [nac.inline_path("/", "首页"),
                 nac.inline_path("/editor/", "写文章"),
                 nac.inline_path("/settings", "设置"),
                 nac.inline_path("/profile/" + user["username"], user["username"])]
    else:
        links = [nac.inline_path("/", "首页"),
                 nac.inline_path("/login", "登录"),
                 nac.inline_path("/register", "注册")]
    with nac.box("nav"):
        with nac.cols(2):
            with nac.box("brand"):
                nac.p("conduit")
            nac.col()
            nac.p(*joined(links))
    nac.gap()


def chip(text):
    return nac.tt(text)


def tabs_row(user, active):
    """两条流之间的切换。当前那条加粗，另一条是链接。"""
    if not user:
        return
    if active == "feed":
        nac.p(nac.b("你关注的"), "        ", nac.inline_path("/", "全部"))
    else:
        nac.p(nac.b("全部"), "        ", nac.inline_path("/feed", "你关注的"))
    nac.gap()


def _opt(row, name, default=None):
    """sqlite3.Row 没有 .get()，取不到的列就当没有。"""
    try:
        value = row[name]
    except (IndexError, KeyError):
        return default
    return default if value is None else value


def byline(row, size=28):
    """头像 + 作者那一行。两栏：左边给头像固定宽度，右边吃掉剩下的。"""
    if True:
        with nac.box():
            nac.avatar(row["author"], size)
        with nac.box():
            with nac.box("meta"):
                bits = []
                if _opt(row, "following"):
                    bits += [chip("已关注"), "  "]
                bits.append(nac.inline_path("/profile/" + row["author"], row["author"]))
                bits.append("   ·   ")
                bits.append(row["created"])
                tags = _opt(row, "tags")
                if tags:
                    first = tags.split(",")[0]
                    bits += ["   ·   ", nac.inline_path("/tag/" + first, "#" + first)]
                favourites = _opt(row, "fav_count")
                if favourites is not None:
                    bits += ["   ·   ♥ " + str(favourites)]
                nac.p(*bits)


def article_card(row):
    with nac.box("card"):
        nac.h3(row["title"])
        with nac.box("lead"):
            nac.p(row["description"])
        nac.gap()
        byline(row)
        nac.gap()
        nac.p(nac.inline_path("/article/" + row["slug"], "读全文 →"))


def feed(rows, empty="还没有文章。"):
    if not rows:
        with nac.box("card"):
            nac.p(nac.i(empty))
        return
    for row in rows:
        article_card(row)


def pager(page, total, base):
    pages = max(1, (total + store.PAGE_SIZE - 1) // store.PAGE_SIZE)
    if pages <= 1:
        return
    with nac.box("card"):
        bits = []
        if page > 1:
            bits.append(nac.inline_path(base if page == 2 else "%s/%d" % (base, page - 1), "← 上一页"))
            bits.append("    ")
        for number in _window(page, pages):
            if number == page:
                bits.append(nac.b(str(number)))
            else:
                bits.append(nac.inline_path(base if number == 1 else "%s/%d" % (base, number), str(number)))
            bits.append("    ")
        if page < pages:
            bits.append(nac.inline_path("%s/%d" % (base, page + 1), "下一页 →"))
        nac.p(*bits)


def _window(page, pages, span=2):
    out = list(range(max(1, page - span), min(pages, page + span) + 1))
    if out and out[0] > 1:
        out = [1] + out
    if out and out[-1] < pages:
        out = out + [pages]
    return sorted(set(out))


def tags_side(tags, current=None):
    with nac.box("side"):
        nac.p(nac.b("热门标签"))
        nac.gap()
        for tag in tags[:10]:
            nac.p(nac.inline_path("/tag/" + tag, tag))
        nac.gap()
        nac.p(nac.i("按标签过滤见 /tag/<名字>。"))


def two_columns(left, right=None):
    if right is None:
        left()
        return
    with nac.cols(2):
        left()
        nac.col()
        right()


def joined(bits):
    """一串条目之间补两个空格。"""
    out = []
    for index, bit in enumerate(bits):
        if index:
            out.append("  ")
        out.append(bit)
    return out


def errors(message):
    if not message:
        return
    with nac.box("card", border="1:#c65252"):
        nac.p(nac.b(message))


def hint(title, body):
    with nac.details(title):
        nac.p(body)


def footer():
    with nac.box("foot"):
        nac.p("conduit · 用 Nac 重写的 RealWorld · ",
              nac.inline_url("https://github.com/gothinkster/realworld", "规格来源"))
    # 关掉 head() 里打开的那个 .page 容器。
    nac.raw("[[end]]")

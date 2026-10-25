"""Conduit 的观感层。

一句话说明这份文件在干什么：**不靠 CSS，靠现有块拼出最接近原版的样子**。

原版 Conduit 的视觉支点只有五个：绿色横幅、居中容器、卡片列表、标签胶囊、页签。
Nac 的客户端不给 CSS，所以逐条翻译成它能画的东西：

  绿色横幅  → `[[scene]]` 的一块绿色矩形（能画色块，但文字不能压在它上面）
  卡片列表  → 标题 + 斜体导语 + 元信息，条目之间用 `---` 的发丝线隔开（原版是 1px 边框）
  标签胶囊  → 行内代码：客户端给它 4px 圆角 + 浅灰底，是全套里最像胶囊的东西
  双栏      → `[[cols 2]]`，左边文章流，右边标签栏（原版是 8:4，这里只能 5:5）
  页签      → 当前项用胶囊，其余用链接（比 `[[tabs]]` 强：`[[tabs]]` 是客户端状态，带不动翻页）

做不到的（客户端没有这个能力，不是页面没写）：自定义字体、白底描边卡片、
居中的最大宽度容器、悬停变色、运行时头像。
"""

import nac

from _lib import store

#: Conduit 的品牌绿。原版是 #5CB85C。
GREEN = "5cb85c"
GREEN_DARK = "449d44"


def band(height=56, dark=10):
    """顶部那条绿色横幅。

    `[[scene]]` 只画矩形，画不了字，所以它纯粹是一条色带 —— 原版那行白色的
    "conduit" 压在色带上，这里只能出现在色带下方。
    """
    width = 1000
    nac.raw("[[scene %dx%d 0.1]]" % (width, height))
    nac.raw("[[bar %s %d %d]]" % (GREEN, width, height - dark))
    nac.raw("[[at 0 0 0]]")
    nac.raw("[[bar %s %d %d]]" % (GREEN_DARK, width, dark))
    nac.raw("[[at 0 0 %d]]" % (height - dark))
    nac.raw("[[end]]")


def hero(tagline):
    """品牌区：色带 + 站名 + 一句副标题。"""
    band()
    nac.h1("conduit")
    nac.p(nac.i(tagline))
    nac.gap()


def chip(text):
    """胶囊：浅灰底、圆角、等宽字。原版是带边框的白底小圆角。"""
    return nac.tt(text)


def nav(user):
    parts = [nac.b("conduit"), "    ", nac.inline_path("/", "首页")]
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
        parts += [
            "  ",
            nac.inline_path("/login", "登录"),
            "  ",
            nac.inline_path("/register", "注册"),
        ]
    nac.p(*parts)
    nac.hr()


def tabs_row(user, active):
    """首页的两条流。当前那条用胶囊，另一条是链接。"""
    if not user:
        return
    if active == "feed":
        nac.p(nac.b("你关注的"), "    ", nac.inline_path("/", "全部"))
    else:
        nac.p(nac.b("全部"), "    ", nac.inline_path("/feed", "你关注的"))
    nac.gap()


def article_card(row):
    nac.h3(row["title"])
    nac.p(nac.i(row["description"]))
    nac.gap()
    bits = []
    if row["following"]:
        bits.append(chip("已关注"))
        bits.append("  ")
    bits.append(nac.inline_path("/profile/" + row["author"], row["author"]))
    bits.append("  ·  ")
    bits.append(row["created"])
    if row["tags"]:
        bits.append("  ")
        bits.append(chip("#" + row["tags"].split(",")[0]))
    bits.append("  ·  ♥ ")
    bits.append(str(row["fav_count"]))
    nac.p(*bits)
    nac.p(nac.inline_path("/article/" + row["slug"], "读全文 →"))
    nac.hr()


def feed(rows, empty="还没有文章。"):
    if not rows:
        nac.gap()
        nac.p(nac.i(empty))
        nac.gap()
        return
    for row in rows:
        article_card(row)


def pager(page, total, base):
    pages = max(1, (total + store.PAGE_SIZE - 1) // store.PAGE_SIZE)
    if pages <= 1:
        return
    bits = []
    if page > 1:
        bits.append(nac.inline_path(base if page == 2 else "%s/%d" % (base, page - 1), "← 上一页"))
        bits.append("  ")
    # 原版是一排数字按钮；这里只能是一排链接，当前页用胶囊标出来。
    for number in _window(page, pages):
        if number == page:
            bits.append(nac.b(str(number)))
        else:
            bits.append(
                nac.inline_path(base if number == 1 else "%s/%d" % (base, number), str(number))
            )
        bits.append("  ")
    if page < pages:
        bits.append(nac.inline_path("%s/%d" % (base, page + 1), "下一页 →"))
    nac.gap()
    nac.p(*bits)


def _window(page, pages, span=2):
    """当前页前后各两页，够用又不至于把一行撑爆。"""
    out = list(range(max(1, page - span), min(pages, page + span) + 1))
    if out and out[0] > 1:
        out = [1] + out
    if out and out[-1] < pages:
        out = out + [pages]
    return sorted(set(out))


def tags_side(tags, current=None):
    """右栏：热门标签。原版是竖排的胶囊按钮。"""
    if not tags:
        return
    nac.h3("热门标签")
    for tag in tags[:12]:
        nac.p(chip("#" + tag), "  ", nac.inline_path("/tag/" + tag, "浏览"))
    nac.gap()
    nac.hr()
    nac.p(nac.i("按标签过滤见 /tag/<名字>。"))


def two_columns(left, right=None):
    """把页面切成「文章流 + 侧栏」。原版是 8:4，这里只有等分。"""
    if right is None:
        left()
        return
    with nac.cols(2):
        left()
        nac.col()
        right()


def joined(bits):
    """一串条目之间补两个空格，拼成一行。"""
    out = []
    for index, bit in enumerate(bits):
        if index:
            out.append("  ")
        out.append(bit)
    return out


def errors(message):
    if not message:
        return
    with nac.danger("没有成功"):
        nac.p(message)


def hint(title, body):
    with nac.details(title):
        nac.p(body)


def footer():
    nac.gap()
    nac.hr()
    nac.p(
        nac.i("conduit · 用 Nac 重写的 RealWorld"),
        "  ",
        nac.inline_url("https://github.com/gothinkster/realworld", "规格来源"),
        "  ·  ",
        nac.i("没有一个字节的 HTML"),
    )

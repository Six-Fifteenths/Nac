# nac:title flappy.bird
# nac:path /

"""首页：一架会自动飞的鸟 + 规则 + 排行榜。

那张演示图是 `[[scene]]` 播的动画（点一下重播）。真正的游戏在 /play，
那边一帧一次请求 —— 为什么，见 /how。
"""

import math
import random

import nac

from _lib import game


def sid():
    """这一位访客的编号，存在会话里，用来记住他的成绩和没打完的那局。"""
    value = nac.session_get("sid")
    if not value:
        value = "%08x" % random.getrandbits(32)
        nac.session_set("sid", value)
    return value


def demo():
    """一段 6 秒的演示：鸟上下摆，水管从右滚到左。

    用的是 `_lib/game.py` 里同一批零件 —— 所以演示和真游戏的画法不会走样。
    """
    out = ["[[scene %dx%d 6.0]]" % (game.FIELD_W, game.FIELD_H)]

    for color, w, h, x, y in game.background():
        out.append("[[bar %s %d %d]]" % (color, w, h))
        out.append("[[at 0 %.1f %.1f]]" % (x, y))

    # 两根水管：6 秒里从右滚到左，零件相对水管左上角
    for index, start_x in enumerate((560.0, 860.0)):
        gap = 70.0 + index * 40
        track = [(step * 0.25, start_x - step * 0.25 * 120.0) for step in range(25)]
        for color, w, h, dx, dy in game.pipe_parts(gap):
            out.append("[[bar %s %d %d]]" % (color, round(w), round(h)))
            for t, x in track:
                out.append("[[at %.2f %.1f %.1f]]" % (t, x + dx, dy))

    # 鸟：三块零件一起摆
    sway = [(step * 0.125, 150.0 + math.sin(step * 0.3) * 42.0) for step in range(49)]
    for color, w, h, dx, dy in game.bird_parts(0.0, tick=0):
        out.append("[[bar %s %d %d]]" % (color, round(w), round(h)))
        for t, y in sway:
            out.append("[[at %.2f %.1f %.1f]]" % (t, game.BIRD_X + dx, y + dy))

    out.append("[[end]]")
    return out


game.ensure_tables(nac)
me = sid()
best = game.best_of(nac, me)

nac.h1("flappy.bird")
nac.p("一只鸟，两根水管，一个按钮。写页面的人只需要 Python。")

nac.write("\n".join(demo()))
nac.p(nac.i("点画面可以重播。这不是游戏本体，按钮在下面。"))
nac.gap()

with nac.cols(2):
    nac.p(nac.b("开始"), "　", nac.inline_path("/play", "去玩一局 →"))
    nac.col()
    nac.p(nac.b("你的最好成绩"), "　", str(best))

nac.gap()
nac.h2("怎么玩")
with nac.note("info", "一次点击 = 一帧"):
    nac.p("拍一下，鸟向上窜一点，然后世界往前走一格。没有实时循环，")
    nac.p("所以这个游戏的节奏，就是你点击的节奏。")
nac.bullets(
    "点「拍一下」，鸟向上飞一点，同时水管向左移一格。",
    "穿过一对水管 +1 分；撞水管、落地、顶到天花板都算结束。",
    "关掉窗口不丢：这一局存在站点的 SQLite 里，下次回来还在。",
)
nac.gap()
nac.p("想知道它为什么长这样（而不是 60 帧的真游戏）：", " ",
      nac.inline_path("/how", "技术说明"))

rows = game.leaderboard(nac, 8)
nac.h2("排行榜")
if not rows:
    nac.p(nac.i("还没有人留下名字。"))
else:
    nac.table(["名字", "分数"], [[row["name"], str(row["best"])] for row in rows])

nac.gap()
nac.hr()
nac.p(nac.i("美术是这个平台画得出来的东西：圆角矩形、纯色、超出画布再裁掉。"))
nac.p(nac.i("没有图片、没有精灵图、没有帧缓冲 —— 只有一块 540×300 的矩形画布。"))

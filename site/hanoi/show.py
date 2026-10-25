# nac:title 汉诺塔 · 演示
# nac:match /hanoi/

# /hanoi/4        4 层，正常速度
# /hanoi/4/slow   慢一档
# /hanoi/4/fast   快一档
#
# 地址决定参数，所以「再来一次」「慢一点」都是普通链接，不需要脚本。

import nac

from _lib import hanoi

SPEEDS = {"slow": 0.45, "fast": 2.2}
SPEED_NAME = {"slow": "慢", "fast": "快"}

arg = nac.path_arg("/hanoi/")
head, _, tail = arg.partition("/")
layers = int(head) if head.isdigit() else 0
speed_key = tail.strip("/")
speed = SPEEDS.get(speed_key, 1.0)

nac.h1("汉诺塔")

if not 1 <= layers <= 7:
    nac.p("层数要在 1 到 7 之间。/hanoi/3 这样写。")
    nac.p(nac.inline_path("/hanoi", "回到起点"))
    nac.stop()

lines, seconds, steps = hanoi.timeline(layers, speed)

nac.p(
    "%d 层 · %d 步 · 约 %.1f 秒" % (layers, steps, seconds),
    "" if not speed_key else " · " + SPEED_NAME.get(speed_key, ""),
)
nac.gap()

# 这里就是那块画面：一行 [[scene]] 开头，若干 [[bar]] 和 [[at]]，[[end]] 收尾。
for line in lines:
    nac.raw(line)

nac.gap()
nac.p(
    nac.inline_path("/hanoi/%d" % layers, "再来一次"),
    " · ",
    nac.inline_path("/hanoi/%d/slow" % layers, "慢一点"),
    " · ",
    nac.inline_path("/hanoi/%d/fast" % layers, "快一点"),
    " · ",
    nac.inline_path("/hanoi/%d" % max(1, layers - 1), "少一层"),
    " · ",
    nac.inline_path("/hanoi/%d" % min(7, layers + 1), "多一层"),
)
nac.p(nac.i("点画面从头播；换速度会重新请求这一页，动画也从头开始。"))

nac.gap()
with nac.details("每一步的顺序"):
    shown = [m for m in hanoi.solve(layers)]
    if len(shown) > 40:
        shown = shown[:40]
    nac.ordered(
        *[
            "第 %d 步：%s 柱 → %s 柱" % (i, "ABC"[a], "ABC"[b])
            for i, (a, b) in enumerate(shown, 1)
        ]
    )
    if steps > len(shown):
        nac.p(nac.i("后面还有 %d 步，没全列出来 —— 一页最多 16 KB。" % (steps - len(shown))))

nac.gap()
with nac.details("递归解法"):
    nac.code(
        "def solve(n, src, dst, spare):\n"
        "    if n == 0:\n"
        "        return\n"
        "    solve(n - 1, src, spare, dst)\n"
        "    move(src, dst)\n"
        "    solve(n - 1, spare, dst, src)",
        "python",
    )
    nac.p(
        "这个函数跑在服务端，每次打开这一页现算。它同时产出「搬哪个盘子」"
        "和「什么时候搬」——客户端不知道汉诺塔是什么，它只会按关键帧画矩形。"
    )

nac.gap()
nac.p(nac.inline_path("/hanoi", "换个层数"), " · ", nac.inline_path("/", "回到 Nac 主站"))

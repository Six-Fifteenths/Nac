# nac:title 汉诺塔
# nac:path /hanoi

# 汉诺塔演示的入口。真正的画在 /hanoi/<层数>，那一页认领了整个 /hanoi/ 地址族。

import nac

from _lib import hanoi

nac.h1("汉诺塔")
nac.p(
    "三根柱子，n 个从大到小的盘子。把整摞从最左搬到最右，"
    "每次只能搬一个，大盘子不能压在小盘子上。"
)

levels = nac.field_of("layers")
if nac.submitted():
    if levels.isdigit() and 1 <= int(levels) <= 7:
        nac.redirect_path("/hanoi/%d" % int(levels))
    else:
        with nac.warn("这个层数不合适"):
            nac.p("1 到 7 之间填一个整数。再往上这一页装不下 —— 页面上限 16 KB。")

nac.gap()
with nac.cols(2):
    nac.h3("开始")
    nac.form_path("/hanoi")
    nac.field("layers", "层数（1–7）")
    nac.button("搭起来")
    nac.col()
    nac.h3("或者")
    for n in (2, 3, 4, 5):
        nac.p(nac.inline_path("/hanoi/%d" % n, "%d 层" % n), " ", nac.i("· %d 步" % (2 ** n - 1)))

nac.gap()
nac.h2("为什么它能动")
nac.p(
    "页面在服务端算出全部 "
    + nac.tt("2^n - 1")
    + " 步，把每一步变成几个关键帧，交给客户端去播放。客户端只做两件事："
    "按时间在关键帧之间插值、把矩形画出来。所以动画是 60 帧的，而服务端只被请求了一次。"
)
nac.gap()
nac.bullets(
    "整摞搬到最右最少要 2^n − 1 步 —— 3 层 7 步，7 层 127 步",
    "递归解法三行：搬走上面 n−1 个、搬最大的、再把 n−1 个搬回来",
    "关键帧之间用 smoothstep 插值，所以起手和收手都是缓的",
)
nac.gap()
nac.p(nac.i("点画面可以重播。"))

nac.gap()
with nac.details("看看这一步是怎么描述的"):
    nac.code(
        "[[scene 640x240 4.06]]\n"
        "[[bar 6b7280 8 176]]\n"
        "[[at 0 106.00 34]]\n"
        "...\n"
        "[[bar c0392b 176 16]]\n"
        "[[at 0.00 22.00 192.00]]\n"
        "[[at 0.19 22.00 34.00]]\n"
        "[[at 0.43 428.00 34.00]]\n"
        "[[at 0.58 428.00 192.00]]\n"
        "[[end]]",
        "text",
    )
    nac.p(
        "一个 " + nac.tt("[[bar]]") + " 是一块矩形（颜色、宽、高），"
        "后面的 " + nac.tt("[[at]]") + " 是它的关键帧（秒、x、y）。"
        "页面想画什么就画什么 —— 汉诺塔只是刚好长这样。"
    )

nac.gap()
nac.p(nac.inline_path("/", "回到 Nac 主站"), " · ", nac.inline_path("/features", "功能一览"))

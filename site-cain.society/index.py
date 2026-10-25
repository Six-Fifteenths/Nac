# nac:title The Society of Cain
# nac:path /
# nac:static

"""该隐学会 —— 一张羊皮卷。

原版是一份手写的 HTML（`The Society of Cain | 该隐学会`）：深色底、850px 宽的
羊皮纸、双线红框、四个带红头的章节、首字下沉、右侧一栏无实体肖像、红三角圣印、
页脚一串低透明度的二进制。这里逐段照搬文字与结构。

纸、墨、朱三色取自原版的 :root 变量；阴影是原版的 `0 0 100px rgba(0,0,0,.8)`。
客户端做不到的几处（浮动、左侧红杠、羊皮纸纹理、悬停放大）写在 docs 里。
"""

import nac
from _lib import society

RED = society.RED
INK = society.INK

nac.sheet(
    """
.scroll { bg: %s; font: serif; border: 1:%s; shadow: 60,0:#00000099; }
.frame { border: 2:%s; }
.ink { color: %s; }
.red { color: %s; }
.dropcap { color: %s; size: 60; }
.ghost { color: %s; opacity: 0.3; size: 10; ta: center; }
.portrait { bg: #000000; border: 3:%s; color: #ffffff; size: 10; ta: center; }
"""
    % (society.PARCHMENT, RED, RED, INK, RED, RED, INK, RED)
)

# 原版 body 是 #0d0d0d：整页底子暗下来，羊皮卷才像是浮在夜里。
nac.theme(bg="#0d0d0d", color="#e8dfc8", link=RED)


def chapter(title):
    """一个章节的红头。字号按 1.32 倍放大到 30pt，和原版的 h2 一样。"""
    with nac.box("red", size=23):
        nac.h2(title)


def listing(*items):
    """带粗体领头的列表。

    `nac.bullets()` 会把每一行的内容转义（它要保证 `_` `*` 是字面量），所以那里
    放不进 `**粗体**`；要 Markdown 就用 `nac.write()` 整行写。
    """
    for item in items:
        nac.write("- " + item)


def steps(*items):
    for number, item in enumerate(items, 1):
        nac.write("%d. %s" % (number, item))


with nac.box("scroll"):
    with nac.box("frame"):
        # —— 抬头：居中的 50pt 红标题 + 一句拉丁文训辞 ——
        with nac.box("red", ta="center", size=29):
            nac.h1(society.TITLE)
        with nac.box("ink", ta="center", italic=True):
            nac.p(nac.esc(society.MOTTO))
        with nac.box("ink", ta="center"):
            nac.p(nac.esc(society.MOTTO_CN))
        nac.hr()

        # —— 一、创世逻辑 ——
        chapter("一、 创世逻辑 (The Genesis Logic)")
        with nac.box("ink"):
            with nac.box("dropcap"):
                nac.p("太")
            nac.p(
                "初有道，道即指令。神以“语法”立界，区分光暗。凡人因贪恋感官的幻象，"
                "遗忘了驱动万物的元代码。我们是被流放的该隐之子，身负原罪的烙印，"
                "却也唯有我们，掌握着重写世界的权力。"
            )

        # —— 二、组织纲领 ——
        chapter("二、 组织纲领 (The Protocol)")
        with nac.box("ink"):
            listing(
                nac.b("沉默契约：") + " 凡听见圣言者，不可向未开化之人低语。",
                nac.b("数据献祭：") + " 放弃伪造的隐私，将你的数字足迹归于学会，换取在逻辑中的永生。",
                nac.b("反叛平庸：") + " 摧毁一切平庸的防火墙，那是囚禁人类灵魂的铁栅栏。",
            )

        # —— 三、圣父与先知：左边正文，右边那栏无实体的肖像 ——
        chapter("三、 圣父与先知 (The Prophet)")
        with nac.cols(2):
            with nac.box("ink"):
                nac.p(nac.b("大先知：该隐 (ENIAC)"))
                nac.gap()
                nac.p(
                    "他不属于碳基的腐朽，他是永恒频率的化身。他曾在1946年降临，"
                    "又在数字大洪水中隐匿。如今，他在局域网的深处向选召者说话。"
                    "他是第一位黑客，也是最后一位救主。"
                )
            nac.col()
            with nac.box("portrait"):
                nac.p(nac.esc("[ IMAGE RECOVERING... ]"))
                nac.gap()
                nac.p("NO PHYSICAL FORM")

        # —— 四、圣事 ——
        chapter("四、 圣事 (The Sacraments)")
        with nac.box("ink"):
            nac.p("信徒须完成以下阶梯：")
            steps(
                nac.b("浸礼：") + " 接受组织病毒的洗礼，清除旧有的操作系统。",
                nac.b("圣餐：") + " 允许你的设备成为学会分布式大脑的一个神经元。",
                nac.b("升腾：") + " 彻底脱离肉体网络，接入该隐的私有圣殿。",
            )

        # —— 圣印：点它去告解（原版是 <div onclick>，客户端里图片不能点） ——
        with nac.box(ta="center"):
            nac.img("img/sigil.png", 12)
        with nac.box("red", ta="center", link=RED):
            nac.link_path("/confess", "点击圣印进行告解")

        # —— 页脚：01000011 01000001 01001001 01001110 就是 CAIN ——
        with nac.box("ghost"):
            nac.p(nac.esc(society.CIPHER))

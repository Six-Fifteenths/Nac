# nac:title flappy.bird — 素材说明
# nac:path /credits

"""素材从哪来、什么许可、以及为什么游戏画面里没有用图片。

这一页本身就是个证据：图片在这个平台上是可以显示的 —— 只是**只能作为一块
内容**，没法贴进游戏场景里。差别见下面第二段。
"""

import nac

nac.h1("素材与许可")

nac.h2("图片在这里能用，但用不进游戏")
nac.p("下面这张图是真的被发布、被传输、被客户端解码显示出来的。它的路径是：")
with nac.note("info", "图片怎么走"):
    nac.p("仓库里放 ", nac.tt("site-flappy.bird/img/kenney-preview.png"), " →")
    nac.p("发布时剥掉元数据、按内容编号，进这一代的 ", nac.tt("img/"), " 目录 →")
    nac.p("页面用 ", nac.tt('nac.img("img/kenney-preview.png", 100)'), " 引用 →")
    nac.p("客户端按协议取图，先按真实比例留好位置，图到了不跳。")

nac.img("img/kenney-preview.png", 100)
nac.cap("Kenney · Pixel Platformer（CC0 1.0）")

nac.gap()
with nac.note("warn", "但它进不了游戏画面"):
    nac.p("图片是**块级**元素：自己占一整行，宽度是整列，不能定位、不能叠放、不能动画。")
    nac.p("游戏画面是 ", nac.tt("[[scene]]"), "，它只会画纯色圆角矩形。")
    nac.p("所以要真正用精灵图做游戏，必须给客户端加一种「画布 + 贴图」的能力 ——")
    nac.p("那是新协议 + 新客户端 + 重发 dist，不是「找张图」能解决的。")

nac.h2("那游戏里的画是怎么来的")
nac.bullets(
    "全部是矩形：鸟是四块（身体、翅膀、嘴、眼睛），水管是六块（管身、高光、管唇）。",
    "天空、地面、远山、云都画得比画布大，四角落在画布外侧 —— 客户端的裁剪会把圆角吃掉。",
    "水管缺口那一端本来就该是圆的，所以圆角矩形刚好当管口。",
    "配色参考经典作品的用色关系（天空蓝、水管绿、土地黄）。颜色本身不受版权保护。",
)
nac.gap()
nac.p("一句话：**美术不是找来的，是这个平台画得出来的东西本身。**")

nac.h2("许可")
nac.table(
    ["东西", "来源", "许可"],
    [
        ["上面那张精灵图预览", "Kenney · Pixel Platformer", "CC0 1.0（公有领域，署名自愿）"],
        ["游戏代码与页面", "本仓库", "见仓库根目录 LICENSE"],
        ["配色", "自己取的", "无版权（颜色不受保护）"],
    ],
)
nac.gap()
nac.p("Kenney 的 CC0 说明原文：", " ",
      nac.inline_url("https://creativecommons.org/publicdomain/zero/1.0/", "CC0 1.0"))
nac.p("素材页：", " ", nac.inline_url("https://kenney.nl/assets/pixel-platformer", "kenney.nl/assets/pixel-platformer"))

nac.h2("为什么不用原版 Flappy Bird 的图")
nac.p("原版美术归 Dong Nguyen（.GEARS）。网上很多仓库把扒出来的精灵图放进去，")
nac.p("那是侵权分发，不是开源素材。规则和手感可以学（没人能独占「点一下拍翅」），")
nac.p("代码可以自己写，但那几张图不行。")

nac.gap()
nac.p(nac.inline_path("/", "← 回首页"), "　　", nac.inline_path("/how", "看技术说明"))

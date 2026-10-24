# nac:title 图片走廊
# nac:path /gallery
# nac:static

import nac

nac.h1("图片走廊")
nac.p("图片不是外链，而是站点目录里的文件；发布时会重新编码并剥掉元数据。")
nac.gap()

nac.img("img/beacon.png", 100)
nac.p(nac.i("beacon.png"), "：灯塔、信号和一点点噪声。")
nac.gap()

with nac.cols(2):
    nac.img("img/palette.png", 100)
    nac.p(nac.i("palette.png"), "：半透明色块叠在一起。")
    nac.col()
    nac.img("img/protocol.png", 100)
    nac.p(nac.i("protocol.png"), "：想象中的二进制帧。")

nac.gap()
nac.h2("小图也不会被拉伸")
nac.p("这张原始图片只有 64×32 像素；客户端按比例预留空间，不会等图片到了才跳版。")
nac.img("img/plan.png", 30)
nac.p(nac.i("plan.png"), "：一张很小的示意图。")
nac.gap()

nac.h2("图片是怎么进来的")
nac.bullets(
    "图片放在 site/img/ 下，发布时从页面里的路径查表。",
    "只接受 png / jpeg / webp / gif；发布工具会剥掉 EXIF 等元数据。",
    "相同字节的图片在相邻两代之间用硬链接，不重复占盘。",
)
nac.gap()
nac.link_path("/features", "功能一览")
nac.link_path("/", "回首页")

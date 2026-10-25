# nac:title 关于 iris.out
# nac:path /about
# nac:static

import nac

nac.h1("关于这个地址")

nac.p(
    "这个站叫 ",
    nac.tt("iris.out"),
    "。它不是 ",
    nac.tt("nac.local"),
    " 底下的一个子域 —— 名字就是名字，中间没有共同的尾巴。",
)

nac.gap()
nac.h2("那客户端怎么找到它")
nac.p(
    "地址栏里写 " + nac.tt("iris.out") + " 的时候，客户端先问当前连着的服务器："
    "「iris.out 在哪？」服务器在自己的注册表里查一条记录，回答一个端点，客户端连过去。"
)
nac.p(
    "所以名字长什么样完全不重要。",
    nac.tt("nac.local"),
    "、",
    nac.tt("shop.nac.local"),
    "、",
    nac.tt("iris.out"),
    "、",
    nac.tt("EX_MYTH"),
    " 走的是同一条路 —— 客户端只做一件事：拿这串字符去查表。",
)

nac.gap()
nac.h2("为什么这样做")
nac.bullets(
    "名字是显示用的，端点才是真的；两者不互相推导",
    "没有中央域名系统，每个服务器自己列它认识谁",
    "加一个站不用重编客户端，也不用改别人",
)

nac.gap()
nac.p(nac.i("这个站自己的数据在它自己的 data 目录里，和主站的不在同一个文件里。"))
nac.gap()
nac.p(nac.inline_path("/", "回清单"))

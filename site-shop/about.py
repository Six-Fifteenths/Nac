# nac:title 关于这个站
# nac:path /about
# nac:static

import nac

nac.h1("这是一个独立的站")

nac.p(
    "它不是你刚才看的那个站。两者唯一的共同点是都由 Nac 提供 —— "
    "各自一个进程、一个端口、一个数据目录、一份证书。"
)

nac.gap()
nac.table(
    ("", "主站", "这个站"),
    [
        ("名字", nac.tt("home"), nac.tt("shop")),
        ("地址", nac.tt("home.nac.local"), nac.tt("shop.nac.local")),
        ("进程", "一个 contentd", "另一个 contentd"),
        ("数据目录", "/srv/nac/data", "/srv/nac-shop/data"),
        ("数据库", "主站自己的", "shop.db，谁也看不见谁"),
        ("能看见对方吗", "不能", "不能"),
    ],
)

nac.gap()
nac.h2("客户端怎么找到它")
nac.p(
    "地址栏里写 " + nac.tt("shop.nac.local") + " 的时候，客户端先问当前连着的服务器："
    "「shop 在哪？」服务器查自己的站点注册表回答一个端点，客户端再连过去。"
)
nac.p(
    "这是一跳的：谁都可以在自己的注册表里登记别人，所以网络可以靠站点互相认识长出来，"
    "不需要一个中央权威。"
)

nac.gap()
nac.p(nac.inline_path("/", "回列表"), " · ", nac.inline_path("/cart", "购物车"))

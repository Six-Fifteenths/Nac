# nac:title 关于这个名字
# nac:path /about
# nac:static

import nac

nac.h1("EX_MYTH 这个名字")

nac.p("它长得不像域名，是因为它不需要像。")

nac.gap()
nac.table(
    ("平时你以为的", "这里实际发生的"),
    [
        ("域名有层级，靠点分隔", "名字就是一整串字符，不拆分"),
        ("后缀必须合法", "只有字母、数字、点、连字符、下划线，别的都不行"),
        ("有权威服务器告诉你 IP", "问当前连着的那个站，它查自己的表"),
        ("改名字要改 DNS 记录并等待", "改注册表一行，立刻生效"),
    ],
)

nac.gap()
nac.h2("具体过程")
nac.ordered(
    "你在地址栏写 EX_MYTH/myth/3",
    "客户端认出 EX_MYTH 是名字、/myth/3 是路径",
    "它问当前服务器：EX_MYTH 在哪",
    "服务器回答一个端点和它自己的名字",
    "客户端连过去；对面自报也叫 EX_MYTH，于是地址栏保留你写的那个名字",
)

nac.gap()
with nac.note("info", "大小写"):
    nac.p(
        "查表不区分大小写：写 ",
        nac.tt("ex_myth"),
        " 也能到，地址栏会换成服务器自报的那个写法。"
        "一个要记住大小写的地址是坏地址。",
    )

nac.gap()
nac.p(nac.inline_path("/", "回索引"))

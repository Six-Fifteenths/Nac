# nac:title 关于 Nac
# nac:path /about
# nac:static

import nac

nac.h1("关于 Nac")
nac.p("Nac 是一个自成一体的发布平台：自己的二进制 TCP 协议、自己的页面格式、自己的客户端。")
nac.p("它不是为了取代网页，而是想验证一条更窄的路：如果内容只在专用客户端里阅读，能省掉多少层？")
nac.gap()

nac.quote("一个页面就是一个 Python 程序；客户端只负责把程序输出的标记排成可读的版面。")
nac.gap()

nac.h2("设计原则")
nac.bullets(
    "没有 HTML、CSS、JavaScript，也没有浏览器引擎。",
    "协议很小：一请求一响应，帧上限 32 KB，页面输出上限 16 KB。",
    "页面脚本超时 2 秒，失败统一返回空响应，不泄露失败原因。",
    "发布是原子的：新代写完才切换 current，失败不会留下半套内容。",
    "公开站使用 TLS 和客户端证书固定；运营台只监听回环。",
)

nac.gap()
nac.h2("技术一览")
with nac.cols(2):
    nac.h3("服务端")
    nac.p("Rust 写的 contentd，静态 musl 二进制；页面由系统 Python 执行。")
    nac.p("发布工具 publish 负责语法检查、图片重编码、静态页预渲染和代切换。")
    nac.col()
    nac.h3("客户端")
    nac.p("Rust + egui 的原生窗口；端点与证书 pin 在构建时嵌入。")
    nac.p("地址栏显示 nac.local + 路径，不显示服务器地址。")

nac.gap()
nac.h2("边界")
nac.bullets(
    "TLS 加密链路，但不隐藏服务器 IP。",
    "没有账号体系；能连上公开站的人都能读取内容、提交表单。",
    "运营台没有登录，安全性依赖回环监听和 SSH 隧道。",
    "页面脚本拥有服务用户的权限，所以只应运行自己写的页面。",
)
nac.gap()
nac.p("许可：仅限非商业用途。")
nac.gap()
nac.link_path("/notes/", "看实现笔记")
nac.link_path("/status", "看运行状态")
nac.link_path("/", "回首页")

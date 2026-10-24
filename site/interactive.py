# nac:title 互动实验
# nac:path /interactive

import nac

name = nac.field_of("name")[:40]
mood = nac.field_of("mood")[:80]
code = nac.field_of("code")
submitted = bool(name or mood or code)

if name:
    nac.session_set("name", name)
if mood:
    nac.session_set("mood", mood)

if submitted:
    visits = nac.counter("interactive-visits.json")
else:
    visits = int((nac.read_json("interactive-visits.json", {"n": 0}) or {}).get("n", 0))

saved_name = nac.session_get("name")
saved_mood = nac.session_get("mood")

nac.h1("互动实验")
nac.p("这一页每次访问都会启动一次 Python，超时 2 秒；表单提交到同一路径。")
nac.gap()

if submitted:
    nac.quote("表单已收到。页面重新执行了一遍，状态从会话和站点数据里读回来。")
    if name:
        nac.p("你叫 ", nac.b(name), "。")
    if mood:
        nac.p("今天想做的事：", nac.i(mood))
    if code:
        if code.strip().lower() == "beacon":
            nac.p(nac.b("暗号正确。"), " 灯塔仍然亮着。")
        else:
            nac.p("暗号不对；提示是 ", nac.tt("beacon"), "。")
    nac.gap()

nac.h2("当前会话")
nac.p("名字：", nac.b(saved_name or "（还没有）"))
nac.p("心情：", nac.i(saved_mood or "（还没有）"))
nac.p("本页累计提交 ", nac.b(visits), " 次。会话存在服务端，关掉客户端再回来仍然在。")
nac.gap()

nac.h2("表单")
nac.form_path("/interactive")
nac.field("name", "你的名字")
nac.field("mood", "今天想做什么")
nac.password("code", "暗号（提示：beacon）")
nac.button("提交到本页")
nac.gap()

nac.h2("用到的接口")
nac.bullets(
    "nac.field_of() 读取字段并去掉首尾空白",
    "nac.session_set() / session_get() 保存跨请求状态",
    "nac.counter() 原子地增加一个计数",
    "nac.form_path() 让表单提交到路径而不是内部页号",
)
nac.gap()
nac.link_path("/guestbook", "去留言板")
nac.link_path("/status", "看看状态页")
nac.link_path("/", "回首页")

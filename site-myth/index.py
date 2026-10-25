# nac:title EX_MYTH
# nac:path /

import nac

from _lib import myth

items = myth.myths()

nac.h1("EX_MYTH")
nac.p("八条。每条一句话，不说教训。")
nac.gap()

for domain, group in myth.domains(items):
    nac.h3(domain)
    lines = []
    for item in group:
        lines.append(
            "%d. %s —— %s"
            % (item["id"], nac.inline_path("/myth/%d" % item["id"], item["name"]), item["line"])
        )
    for line in lines:
        nac.raw(line)
    nac.gap()

with nac.note("warn", "这个站的名字里有下划线"):
    nac.p(
        "地址是 ",
        nac.tt("EX_MYTH"),
        "。下划线在大多数域名系统里是非法的，但在这里它只是一串字符 —— "
        "服务器按名字回答，客户端按名字查表，没有人去校验它像不像域名。",
    )

nac.gap()
nac.p(nac.inline_path("/about", "这个名字是怎么被找到的"), " · ", nac.inline_path("/", "去主站"))

# nac:title 文字工具箱
# nac:path /toolbox

import hashlib

import nac

text = nac.field_of("text")

nac.h1("文字工具箱")
nac.p("一个小工具：把文本交给 Python，算完把结果画成表格。每次提交都会重新执行这一页。")
nac.gap()

nac.form_path("/toolbox")
nac.textarea("text", "把文字粘进来")
nac.button("算一次")
nac.gap()

if nac.submitted():
    if not text:
        nac.quote("输入是空的，先写点东西。")
    else:
        lines = text.splitlines() or [""]
        han = sum(1 for ch in text if "\u4e00" <= ch <= "\u9fff")
        words = len(text.split())
        nac.h2("统计")
        nac.table(
            ("指标", "数值"),
            [
                ("字符数", nac.b(len(text))),
                ("不含空格", len(text.replace(" ", "").replace("\n", ""))),
                ("行数", len(lines)),
                ("词数（按空格切）", words),
                ("汉字数", han),
                ("UTF-8 字节", len(text.encode("utf-8"))),
                ("SHA-256", nac.tt(hashlib.sha256(text.encode("utf-8")).hexdigest()[:16] + "…")),
            ],
        )

        nac.h2("预览")
        with nac.tabs("原样", "大写", "反转"):
            nac.code(text[:400])
            nac.tab()
            nac.code(text[:400].upper())
            nac.tab()
            nac.code(text[:400][::-1])

        with nac.tip("这些都在服务器上算"):
            nac.p("页面拿到你的输入，调用标准库 ", nac.tt("hashlib"), " 和内置的字符串方法，把结果拼成标记发回来。")
            nac.p("没有 JavaScript，客户端只负责显示。")
else:
    nac.p("上面输入框里写点东西，按“算一次”。")

nac.gap()
nac.nav("/", "/toolbox")
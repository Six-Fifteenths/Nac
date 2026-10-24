# nac:title 页面标记
# nac:path /notes/markup
# nac:static

import nac

nac.h1("页面标记")
nac.p("页面脚本最终输出的是一行行标记；客户端只认这些标记，不执行 HTML。")
nac.gap()

nac.h2("一行一个块")
nac.code(
    "# 一级标题\n"
    "## 二级标题\n"
    "> 引用\n"
    "- 列表项\n"
    "| 等宽代码行\n"
    "!img 1 80% 960x540\n"
    "[[path:/notes/|笔记目录]]\n"
    "[[form 7]]\n"
    "[[input name|名字]]\n"
    "[[button 提交]]\n"
    "[[cols 2]] ... [[end]]"
)
nac.gap()

nac.h2("通常不用手写")
nac.p("大部分标记由 ", nac.tt("nac"), " 辅助库生成：", nac.tt("h1/p/quote/bullets/code/img/link_path/form_path/cols"), "。")
nac.p("需要兜底时，用 ", nac.tt("nac.raw()"), " 原样写一行。")
nac.gap()

nac.h2("为什么不做 HTML")
nac.bullets(
    "客户端不需要浏览器引擎，体积和攻击面都更小。",
    "标记种类有限，排版结果可预测。",
    "不执行脚本，页面只能输出内容。",
)
nac.gap()
nac.link_path("/notes/", "回笔记")
nac.link_path("/", "回首页")

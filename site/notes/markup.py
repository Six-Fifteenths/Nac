# nac:title 页面标记
# nac:path /notes/markup
# nac:static

import nac

nac.h1("页面标记")
nac.p("这一页讲最底层的行标记——也就是 ", nac.tt("nac"), " 最终写给客户端的东西。")
nac.p("平时用辅助函数就够了；标记本身值得知道，因为 ", nac.tt("nac.raw()"), " 可以直接写它。")
nac.gap()

with nac.tip("先看结论"):
    nac.p("标记是按行解析的，每种标记占一整行；只有行内样式和行内链接可以出现在段落中间。")

nac.gap()
nac.h2("块级标记")
nac.table(
    ("写法", "是什么"),
    [
        (nac.tt("# 文字"), "一级标题"),
        (nac.tt("## 文字"), "二级标题"),
        (nac.tt("### 文字"), "三级标题"),
        (nac.tt("普通一行"), "段落，连续的行会合并"),
        (nac.tt("> 文字"), "引用，连续的合成一块"),
        (nac.tt("- 文字"), "无序列表"),
        (nac.tt("1. 文字"), "有序列表，编号由客户端重排"),
        (nac.tt("| 文字"), "代码块（等宽、保留缩进）"),
        (nac.tt("---"), "分隔线"),
        (nac.tt("空行"), "分段"),
    ],
)

nac.h2("行内标记")
nac.table(
    ("写法", "效果"),
    [
        (nac.tt("*文字*"), "粗体"),
        (nac.tt("_文字_"), "斜体"),
        (nac.tt("`文字`"), "等宽"),
        (nac.tt("\\*"), "反斜杠转义，原样显示"),
        (nac.tt("[[path:/x|文字]]"), "行内链接，不独占一行"),
    ],
)
nac.p("实测：", "*粗*", "、", "_斜_", "、", "`等宽`", "，还有一段 ", "[[path:/features|行内链接]]", " 接在后面。")

nac.gap()
nac.h2("容器")
nac.p("容器用 ", nac.tt("[[名字 ...]]"), " 打开，用 ", nac.tt("[[end]]"), " 关闭，中间可以再放容器。")
nac.table(
    ("打开", "关闭 / 分隔", "说明"),
    [
        (nac.tt("[[cols 2]]"), nac.tt("[[col]] · [[end]]"), "并排分栏，最多 4 列"),
        (nac.tt("[[note warn|标题]]"), nac.tt("[[end]]"), "提示框，kind 取 info/ok/warn/danger"),
        (nac.tt("[[details 标题]]"), nac.tt("[[end]]"), "可折叠区块"),
        (nac.tt("[[tabs 甲;乙]]"), nac.tt("[[tab]] · [[end]]"), "页签"),
        (nac.tt("[[table]]"), nac.tt("[[end]]"), "表格，第一行是表头"),
        (nac.tt("[[code python]]"), nac.tt("[[end]]"), "带语言名的代码块"),
    ],
)
with nac.warn("[[end]] 必须对得上"):
    nac.p("少写一个，后面的内容会被吞进容器里。多写一个会被忽略（当空行处理）。")

nac.gap()
nac.h2("表单")
nac.table(
    ("标记", "控件"),
    [
        (nac.tt("[[form 页号]]"), "声明提交目标"),
        (nac.tt("[[input 名|提示]]"), "单行输入"),
        (nac.tt("[[secret 名|提示]]"), "密码输入"),
        (nac.tt("[[textarea 名|提示]]"), "多行输入"),
        (nac.tt("[[select 名|提示|甲;乙]]"), "下拉选择"),
        (nac.tt("[[check 名|标签]]"), "复选框"),
        (nac.tt("[[button 文字]]"), "提交按钮"),
    ],
)

nac.gap()
nac.h2("图片与说明")
nac.code("!img 3 80% 640x360\n!cap 一张图的说明", lang="text")
nac.p("三个数字是图片 id、显示宽度百分比、原始宽高；客户端靠最后一个在图片到达前预留位置。")

nac.gap()
nac.h2("版本不一致会怎样")
with nac.danger("未知指令不会被吞掉"):
    nac.p("如果客户端不认识某个 ", nac.tt("[["), " 指令，它会把整行当普通文字显示。")
    nac.p("所以升级客户端之前，新标记会明明白白露出来，而不是悄悄消失。")

nac.gap()
nac.nav("/", "/notes/", "/notes/markup", "/notes/paths")
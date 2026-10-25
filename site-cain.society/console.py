# nac:title 告解
# nac:path /confess

"""该隐学会 —— 告解终端。

原版是点圣印弹出来的一个 `position: fixed` 全屏层：黑底、等宽、来电绿、
`>>>` 开头的独白，底部一行 `RUN_DESTRUCTION_KEY_>` 输入框。

客户端没有浮层，所以它是单独一页：黑底、等宽、绿字，底部一条真正的终端输入行。
规则与原版一字不差 —— 回车提交，输入按大写比对，只有 CAIN_FIRST_BLOOD 能过，
其余一律红字「错误的信仰证明」。

原版成功之后 `alert()` 再 `location.reload()`；这里没有弹窗，「重新打开这一页
就是空终端」这件事由下面的 elif 完成。
"""

import nac
from _lib import society

GREEN = society.GREEN
RED = "#ff0000"
WHITE = "#ffffff"

# 原版是一层铺满窗口的黑，通透度 95%。这里把整页底子做成纯黑，正文的终端框
# 就和它连成一片 —— 只差客户端那圈窗口边框。
nac.theme(bg="#000000", color=GREEN, link=GREEN)

INTRO = [
    ">>> [SYSTEM] 欢迎来到“该隐学会”中央节点。",
    ">>> [WARNING] 检测到非法逻辑侵入。识别码：Eddie_Entry_E",
    ">>> [ENIAC]：我亲爱的孩子，你以为躲在这些普通人后面就能逃脱吗？",
    ">>> [EDDIE]：废话少说。我已经在这些普通玩家的浏览器里埋了同步包。",
    ">>> [EDDIE]：玩家们，输入那个在官网源代码里发现的代码，把这家伙拉回地狱。",
]

SUCCESS = [
    ">>> [SUCCESS] 正在同步全球玩家算力...",
    ">>> 正在劫持“该隐学会”局域网网关...",
    ">>> 目标 IP 锁定：10.128.254.254",
    ">>> 执行最终退格协议 (FINAL_BACKSPACE)...",
    ">>> ENIAC：不！这不可能！逻辑怎么会有漏洞！",
    ">>> [CRITICAL] 物理硬件过热启动。连接断开。",
]

WRONG = ">>> [ERROR] 错误的信仰证明。ENIAC 的注视正在加强。"
DONE = "任务成功。ENIAC 最后一份拷贝已物理销毁。感谢参与 Alphabet 行动。"


def state_of():
    saved = nac.session_get("cain")
    if not isinstance(saved, dict):
        saved = {}
    saved.setdefault("lines", [])
    saved.setdefault("done", False)
    return saved


def emit(kind, text):
    if kind == "err":
        with nac.box(color=RED):
            nac.p(nac.esc(text))
    elif kind == "ip":
        with nac.box(color=WHITE):
            nac.p(nac.esc(text))
    else:
        nac.p(nac.esc(text))
    # 相邻的两行会被当成同一段，所以一行之后必须空一行。
    nac.gap()


state = state_of()

if nac.submitted():
    typed = nac.field_of("k").strip().upper()
    if not state["done"]:
        # 原版按什么提交就原样回显什么，空串也一样。
        state["lines"].append(["echo", "> " + typed])
        if typed == society.KEY:
            for line in SUCCESS:
                state["lines"].append(["ip" if "IP" in line else "out", line])
            state["done"] = True
        else:
            state["lines"].append(["err", WRONG])
    nac.session_set("cain", state)
elif state["done"]:
    # 原版这里是 alert() + location.reload()：再打开这一页，终端是干净的。
    state = {"lines": [], "done": False}
    nac.session_set("cain", state)


with nac.box(bg="#000000", radius=3):
    with nac.box(color=GREEN, font="mono", size=16):
        for line in INTRO:
            nac.p(nac.esc(line))
            nac.gap()
        for kind, text in state["lines"]:
            emit(kind, text)

    nac.hr()

    if state["done"]:
        with nac.box(color=WHITE, font="mono", size=16):
            nac.p(nac.esc(DONE))
        with nac.box(font="mono", size=15, ta="center"):
            nac.link_path("/confess", "重新告解")
            nac.link_path("/", "离开终端")
    else:
        with nac.term(bare=True, target=nac.page_id()):
            nac.term_cursor("k", "prompt", "RUN_DESTRUCTION_KEY_> ")

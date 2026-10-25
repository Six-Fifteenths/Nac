# nac:title WebShell
# nac:path /

"""WebShell 的 Nac 复刻。

原版：https://github.com/nasan016/webshell （MIT，作者 nasan016）。
这里是重写而不是移植：原版是 TypeScript + CSS，这里是一个 Python 页面。
配色、排版、命令、彩蛋都照原版做；做不到的地方写在页面末尾。
"""

import random

import nac

USER = "visitor"
HOST = "webshell"
PASSWORD = "050823"
REPO = "https://github.com/nasan016/webshell"

ART = [
    "███████╗██╗  ██╗███████╗██╗     ██╗",
    "██╔════╝██║  ██║██╔════╝██║     ██║",
    "███████╗███████║█████╗  ██║     ██║",
    "╚════██║██╔══██║██╔══╝  ██║     ██║",
    "███████║██║  ██║███████╗███████╗███████╗",
    "╚══════╝╚═╝  ╚═╝╚══════╝╚══════╝╚══════╝",
]

WHOAMI = [
    ["In the kaleidoscope of existence,", "I am but a reflection questioning the enigma - "],
    ["Amidst cosmic whispers,", "I navigate the maze of self-discovery,", "echoing the eternal refrain - "],
    ["In the symphony of life,", "I am a note inquiring its own melody,", "harmonizing with the universal query - "],
    ["As stardust contemplating its journey,", "I ponder the cosmic query,", "silently asking - "],
    ["In the tapestry of reality,", "I am the thread of self-inquiry,", "weaving through the eternal question - "],
]

HELP = [
    ("'about'", "Who made this website?"),
    ("'projects'", "Maybe there's something interesting."),
    ("'whoami'", "A perplexing question."),
    ("'sudo'", "???"),
    ("'repo'", "View the Github Repository."),
    ("'banner'", "Display the banner."),
    ("'clear'", "Clear the terminal."),
]

PROJECTS = [
    ("GofeR", "Vue.js reactivity in Go.", "https://github.com/nasan016/gofer"),
    ("WebShell", "Terminal styled website.", REPO),
]


# --------------------------------------------------------------- 输出小工具


def out(*text):
    return [("out", t) for t in text]


def blank():
    return [("out", "")]


def banner_lines():
    lines = [blank()]
    for row in ART:
        lines.append([("ascii", row)])
    lines += [
        blank(),
        out("Welcome to WebShell v1.0.0"),
        [("out", "Type "), ("cmd", "'help'"), ("out", " for a list of all available commands.")],
        [("out", "Type "), ("cmd", "'repo'"), ("out", " to view the GitHub repository.")],
        blank(),
    ]
    return lines


def pad(name, width=15):
    return " " * max(2, width - len(name))


# ------------------------------------------------------------------ 各条命令


def run_help():
    lines = [blank()]
    for name, what in HELP:
        lines.append([("out", "  "), ("cmd", name), ("out", pad(name) + what)])
    lines += [
        blank(),
        [("out", "Press "), ("keys", "[Enter]"), ("out", " to run a command.")],
        [("out", "Type "), ("cmd", "'clear'"), ("out", " to start over.")],
        blank(),
    ]
    return lines


def run_about():
    return (
        [blank(), out("Hi stranger. Welcome to WebShell."), blank()]
        + [
            [("out", "  Github   "), ("link", "https://github.com/nasan016")],
            [("out", "  Project  "), ("link", REPO)],
        ]
        + [blank()]
    )


def run_projects():
    lines = [blank()]
    for name, what, url in PROJECTS:
        lines.append([("link", url), ("out", pad(name) + what)])
    lines += [blank(), out("%d File(s)" % len(PROJECTS)), blank()]
    return lines


def run_whoami():
    poem = random.choice(WHOAMI)
    lines = [blank()]
    for index, row in enumerate(poem):
        if index == len(poem) - 1:
            lines.append([("out", row), ("cmd", "who am I?")])
        else:
            lines.append(out(row))
    lines.append(blank())
    return lines


def run_default():
    return [
        blank(),
        out("COMMAND NOT FOUND"),
        [("out", "Type "), ("cmd", "'help'"), ("out", " to get started.")],
        blank(),
    ]


def dispatch(command, state):
    """跑一条命令，返回 (要追加的输出行, 额外动作)。"""
    if command.startswith("rm -rf") and command.strip() != "rm -rf":
        if not state["sudo"]:
            return [blank(), out("Permission not granted."), blank()], None
        if command == "rm -rf src":
            if state["bare"]:
                return [blank(), out("there's no more src folder."), blank()], None
            state["bare"] = True
            return (
                [blank(), out("What made you think that was a good idea?"), blank()]
                + [blank(), out("Now everything is ruined."), blank()]
            ), "bare"
        if state["bare"]:
            return [blank(), out("What else are you trying to delete?"), blank()], None
        return [
            blank(),
            out("Directory not found."),
            [("out", "type "), ("cmd", "'ls'"), ("out", " for a list of directories.")],
            blank(),
        ], None

    if command == "clear":
        return [], "clear"
    if command == "banner":
        return ([blank(), out("WebShell v1.0.0"), blank()] if state["bare"] else banner_lines()), None
    if command == "help":
        return (
            [blank(), out("maybe restarting your browser will fix this."), blank()]
            if state["bare"]
            else run_help()
        ), None
    if command == "whoami":
        return ([blank(), out(USER), blank()] if state["bare"] else run_whoami()), None
    if command == "about":
        return ([blank(), out("Nothing to see here."), blank()] if state["bare"] else run_about()), None
    if command == "projects":
        return (
            [blank(), out("I don't want you to break the other projects."), blank()]
            if state["bare"]
            else run_projects()
        ), None
    if command == "repo":
        return [blank(), out("Redirecting to github.com..."), blank(), [("link", REPO)]], None
    if command == "sudo":
        if state["bare"]:
            return [blank(), out("no."), blank()], None
        return [], "ask-password"
    if command == "ls":
        if state["bare"]:
            return [blank()], None
        if state["sudo"]:
            return [blank(), out("src"), blank()], None
        return [blank(), out("Permission not granted."), blank()], None
    if command == "rm -rf":
        if state["bare"]:
            return [blank(), out("don't try again."), blank()], None
        if state["sudo"]:
            return [blank(), [("out", "Usage: "), ("cmd", "'rm -rf <dir>'")], blank()], None
        return [blank(), out("Permission not granted."), blank()], None
    return run_default(), None


# ---------------------------------------------------------------------- 页面


def state_of():
    saved = nac.session_get("webshell") or {}
    if not isinstance(saved, dict):
        saved = {}
    saved.setdefault("records", [])
    saved.setdefault("sudo", False)
    saved.setdefault("bare", False)
    saved.setdefault("waiting", False)
    saved.setdefault("tries", 0)
    saved.setdefault("started", False)
    return saved


def push(state, command, lines):
    if lines:
        state["records"].append({"cmd": command, "out": lines})
    while len(state["records"]) > 12:
        state["records"].pop(0)


def handle_submit(state):
    if state["waiting"]:
        typed = nac.field_of("pw")
        state["waiting"] = False
        if typed == PASSWORD:
            state["sudo"] = True
            state["tries"] = 0
            push(state, "", [blank(), out("PERMISSION GRANTED."), [("out", "Try "), ("cmd", "'rm -rf'")], blank()])
        elif state["tries"] + 1 >= 2:
            state["tries"] = 0
            push(state, "", [blank(), out("INCORRECT PASSWORD."), out("PERMISSION NOT GRANTED."), blank()])
        else:
            state["tries"] += 1
        return

    command = nac.field_of("cmd")
    if not command.strip():
        return

    lowered = command.lower().strip()
    lines, action = dispatch(lowered, state)
    if action == "clear":
        state["records"] = []
        return
    if action == "ask-password":
        state["waiting"] = True
        push(state, command, [blank()])
        return
    if action == "bare":
        state["records"] = []
    push(state, command, lines)


def prompt_line(*tail):
    return (
        ("pu", USER),
        ("pd", "@"),
        ("ph", HOST),
        ("pd", ":$ ~ "),
    ) + tuple(tail)


def emit(line):
    if not line:
        nac.term_blank()
        return
    if len(line) == 1 and line[0][0] == "ascii":
        nac.term_ascii(line[0][1])
        return
    pairs = []
    for role, text in line:
        pairs.extend([role, text])
    nac.term_line(*pairs)


state = state_of()
if not state["started"]:
    # 横幅是滚动区里的第一条，和原版一样会一直留在上面，直到被顶出去。
    state["records"].append({"cmd": "", "out": banner_lines()})
    state["started"] = True
    nac.session_set("webshell", state)

if nac.submitted():
    handle_submit(state)
    nac.session_set("webshell", state)
    nac.redirect_path("/")

state = state_of()

with nac.term("WebShell.x64_x86", target=nac.page_id(), bare=state["bare"]):
    for record in state["records"]:
        if record["cmd"]:
            nac.term_line(*prompt_line(("in", record["cmd"])))
        else:
            nac.term_line(*prompt_line())
        for line in record["out"]:
            emit(line)
    if state["waiting"]:
        nac.term_passwd("pw", "out", "Password: ")
    else:
        nac.term_cursor("cmd", *prompt_line())

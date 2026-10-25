"""WebShell 这个壳的全部内容。

`index.py` 是会跑命令的那段逻辑，这里只放"说哪些话"。换一份这个文件就是
另一个壳：名字、密码、横幅、帮助、项目、诗句都换掉，代码一行都不用动。

原版：https://github.com/nasan016/webshell （MIT，作者 nasan016）
"""

USER = "visitor"
HOST = "webshell"
TITLE = "WebShell.x64_x86"
VERSION = "v1.0.0"
PASSWORD = "050823"
REPO = "https://github.com/nasan016/webshell"

#: 提示符的分段。角色名见 nac.TERM。
PROMPT = [
    ("user", USER),
    ("prompt", "@"),
    ("host", HOST),
    ("prompt", ":$ ~ "),
]

ART = [
    "███████╗██╗  ██╗███████╗██╗     ██╗",
    "██╔════╝██║  ██║██╔════╝██║     ██║",
    "███████╗███████║█████╗  ██║     ██║",
    "╚════██║██╔══██║██╔══╝  ██║     ██║",
    "███████║██║  ██║███████╗███████╗███████╗",
    "╚══════╝╚═╝  ╚═╝╚══════╝╚══════╝╚══════╝",
]

#: 横幅后面那几行。每行是一串 (角色, 文字)。
WELCOME = [
    [("out", "Welcome to WebShell " + VERSION)],
    [("out", "Type "), ("cmd", "'help'"), ("out", " for a list of all available commands.")],
    [("out", "Type "), ("cmd", "'repo'"), ("out", " to view the GitHub repository.")],
]

#: `whoami` 每次随机挑一首。
WHOAMI = [
    ["In the kaleidoscope of existence,", "I am but a reflection questioning the enigma - "],
    ["Amidst cosmic whispers,", "I navigate the maze of self-discovery,", "echoing the eternal refrain - "],
    ["In the symphony of life,", "I am a note inquiring its own melody,", "harmonizing with the universal query - "],
    ["As stardust contemplating its journey,", "I ponder the cosmic query,", "silently asking - "],
    ["In the tapestry of reality,", "I am the thread of self-inquiry,", "weaving through the eternal question - "],
]

#: `help` 的输出：(命令, 说明)。
HELP = [
    ("'about'", "Who made this website?"),
    ("'projects'", "Maybe there's something interesting."),
    ("'whoami'", "A perplexing question."),
    ("'sudo'", "???"),
    ("'repo'", "View the Github Repository."),
    ("'banner'", "Display the banner."),
    ("'clear'", "Clear the terminal."),
]

#: `projects` 的输出：(名字, 说明, 链接)。
PROJECTS = [
    ("GofeR", "Vue.js reactivity in Go.", "https://github.com/nasan016/gofer"),
    ("WebShell", "Terminal styled website.", REPO),
]

# nac:title 运营台 · 概览
# nac:path /
# 这个站点只在本机 7001 上跑，用 ssh -L 转发到你自己的电脑上看。
# NAC_ROOT 由 contentd --env 传进来，指向工作区根目录。

import os
import time
from pathlib import Path

import nac

root = Path(os.environ.get("NAC_ROOT") or ".")
data = root / "data"
site = root / "site"


def size_of(path):
    total = 0
    if path.exists():
        for item in path.rglob("*"):
            if item.is_file():
                total += item.stat().st_size
    return total


def human(n):
    for unit in ("B", "KB", "MB", "GB"):
        if n < 1024:
            return "%.0f %s" % (n, unit)
        n /= 1024.0
    return "%.1f TB" % n


gen_file = data / "current"
gen = gen_file.read_text(encoding="utf-8").strip() if gen_file.exists() else "（没有发布）"
gen_dir = data / "gen" / gen
sessions = data / "state" / "sessions"
appdata = data / "appdata"
log_path = os.environ.get("NAC_LOG")

nac.h1("运营台")
nac.p("根目录 ", nac.tt(root))
nac.gap()

nac.h2("站点")
nac.bullets(
    "当前代：" + gen,
    "源页面：" + str(len(list(site.glob("*.py")))) + " 个 .py",
    "产物占用：" + human(size_of(gen_dir)),
)
nac.gap()

nac.h2("运行数据")
nac.bullets(
    "会话文件：" + str(len(list(sessions.glob("*.json"))) if sessions.exists() else 0) + " 个",
    "appdata：" + human(size_of(appdata)),
    "访问日志：" + (str(log_path) if log_path else "未配置（NAC_LOG）"),
    "服务器时间：" + time.strftime("%Y-%m-%d %H:%M:%S"),
)
nac.gap()

nac.h2("快捷")
nac.link_path("/publish", "发布新代")
nac.link_path("/data", "看数据 / 清留言")
nac.link_path("/inventory", "内容清单")
nac.link_path("/stats", "访问统计")

nac.done()

# nac:title 运营台 · 访问统计
# nac:path /stats
# 读 contentd --log 写出的 JSONL，在浏览器... 不，在 viewer 里看汇总。
# 日志是追加写的，这里只读不改。

import json
import os
import time
from collections import Counter
from pathlib import Path

import nac

log_path = os.environ.get("NAC_LOG")
root = Path(os.environ.get("NAC_ROOT") or ".")
page_paths = {}
try:
    gen = (root / "data" / "current").read_text(encoding="utf-8").strip()
    manifest = json.loads((root / "data" / "gen" / gen / "manifest.json").read_text(encoding="utf-8"))
    for page in manifest.get("pages", []):
        page_paths[int(page["id"])] = page.get("path") or "/%s" % page["id"]
except Exception:
    pass

nac.h1("访问统计")

if not log_path:
    nac.quote("没有配置 NAC_LOG：启动运营台时加 --env NAC_LOG=/path/to/access.jsonl")
    nac.link_path("/", "回概览")
    nac.done()
    raise SystemExit(0)

path = Path(log_path)
if not path.exists():
    nac.quote("日志文件还不存在：" + str(path))
    nac.p("公开站点要用 --log 启动一次才会产生访问记录。")
    nac.link_path("/", "回概览")
    nac.done()
    raise SystemExit(0)

today = time.strftime("%Y-%m-%d")
total = 0
today_total = 0
pages = Counter()
results = Counter()
sessions = set()
slowest = (0, "")
bad = 0

with open(path, "r", encoding="utf-8", errors="replace") as fh:
    for line in fh:
        line = line.strip()
        if not line:
            continue
        try:
            row = json.loads(line)
        except ValueError:
            bad += 1
            continue
        total += 1
        stamp = int(row.get("t") or 0)
        day = time.strftime("%Y-%m-%d", time.localtime(stamp))
        if day == today:
            today_total += 1
        result = str(row.get("result") or "?")
        results[result.split(":")[0]] += 1
        page = row.get("page")
        if page is not None and result.startswith("ok:"):
            pages[int(page)] += 1
        sess = row.get("sess")
        if sess:
            sessions.add(sess)
        ms = int(row.get("ms") or 0)
        if ms > slowest[0]:
            slowest = (ms, "%s · %s" % (time.strftime("%m-%d %H:%M", time.localtime(stamp)), page_paths.get(page, "/%s" % page)))

nac.p("今天 ", nac.b(today_total), " 次请求；累计 ", nac.b(total), " 次；独立会话 ", nac.b(len(sessions)))
if slowest[0]:
    nac.p("最慢一次 ", nac.tt(str(slowest[0]) + " ms"), "（" + slowest[1] + "）")
if bad:
    nac.p("有 ", str(bad), " 行日志解析失败")
nac.gap()

nac.h2("结果分布")
fail = results.get("fail", 0) + results.get("malformed", 0) + results.get("no-generation", 0)
share = (100.0 * fail / total) if total else 0.0
for key, count in results.most_common():
    nac.bullets("%s：%d" % (key, count))
nac.p("失败占比 ", nac.b("%.1f%%" % share))
nac.gap()

nac.h2("被看得最多的页")
if pages:
    for page_id, count in pages.most_common(10):
        nac.bullets("%s：%d 次" % (page_paths.get(page_id, "/%d" % page_id), count))
else:
    nac.p("还没有成功的取页记录。")
nac.gap()

nac.link_path("/", "回概览")
nac.done()

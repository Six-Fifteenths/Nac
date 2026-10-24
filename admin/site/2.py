# nac:title 运营台 · 发布
# nac:path /publish
# 点一下就在服务器上跑 publish：校验全部页面，通过了才切换新一代。

import os
import subprocess
from pathlib import Path

import nac


def publish_bin(root):
    explicit = os.environ.get("NAC_PUBLISH")
    if explicit:
        return explicit
    for rel in ("target/release/publish", "target/debug/publish"):
        base = root / rel
        for candidate in (base.with_name(base.name + ".exe"), base):
            if candidate.exists():
                return str(candidate)
    return "publish"


root = Path(os.environ.get("NAC_ROOT") or ".")
result = nac.session().get("last_publish")

if "go" in nac.fields():
    try:
        done = subprocess.run(
            [
                publish_bin(root),
                "--site",
                str(root / "site"),
                "--data",
                str(root / "data"),
                "--archive",
                str(root / "archive"),
            ],
            capture_output=True,
            timeout=1.5,
            cwd=str(root),
        )
        text = (done.stdout + done.stderr).decode("utf-8", "replace").strip()
        result = {
            "ok": done.returncode == 0,
            "text": text[-1200:] or "（无输出）",
        }
    except subprocess.TimeoutExpired:
        result = {"ok": False, "text": "publish 超时（脚本只有 2 秒）"}
    except Exception as exc:
        result = {"ok": False, "text": "%s: %s" % (type(exc).__name__, exc)}

    state = nac.session()
    state["last_publish"] = result
    nac.save_session(state)

nac.h1("发布")
if result:
    nac.p(nac.b("上次结果："), "成功" if result["ok"] else "失败")
    nac.gap()
    nac.code(result["text"])
else:
    nac.p("还没有从运营台发布过。")
nac.gap()

nac.form_path("/publish")
nac.field("go", "写点什么都行，然后按下面的按钮")
nac.button("发布")
nac.gap()
nac.p(nac.i("校验在你按下按钮的那一秒完成：页面语法错、.page 残留、图片缺失都会当场失败，旧的一代继续服务。"))
nac.gap()
nac.link_path("/", "回概览")

nac.done()

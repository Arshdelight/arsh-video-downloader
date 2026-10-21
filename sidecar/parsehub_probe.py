#!/usr/bin/env python
"""ArshVideoDownloader 解析 sidecar。

用法: parsehub-probe.exe <url>
输出: stdout 单行 JSON —— {"ok": true, "platform": ..., "title": ..., "video_url": ...}
      失败时 {"ok": false, "error": "..."}

基于 ParseHub (https://github.com/z-mio/ParseHub, MIT)，
抖音等平台的反爬签名由上游库维护，本应用不自行实现。
"""

import json
import sys
import types

# PyInstaller bootloader 忽略 PYTHONUTF8/PYTHONIOENCODING 环境变量，
# stdout 默认随系统 ANSI（中文 Windows = GBK）；中文标题必须强制 UTF-8，
# 否则调用方按 UTF-8 解码会得到乱码文件名
for _stream in (sys.stdout, sys.stderr):
    _stream.reconfigure(encoding="utf-8")


def _install_stubs() -> None:
    """注入 cv2/PIL 空壳模块。

    parsehub 的 parser 注册是 eager import，会经 utils/media_info.py 拉起
    cv2+numpy+PIL（约 150MB）；而这三者只服务于读取微博/Twitter 图集
    落盘后的文件信息，探针流程不会执行该路径。配合打包时的
    --exclude-module 排除真库。
    """
    cv2 = types.ModuleType("cv2")
    cv2.VideoCapture = object
    cv2.CAP_PROP_FRAME_WIDTH = 0
    cv2.CAP_PROP_FRAME_HEIGHT = 0
    cv2.CAP_PROP_FPS = 0
    cv2.CAP_PROP_FRAME_COUNT = 0
    pil = types.ModuleType("PIL")
    pil_image = types.ModuleType("PIL.Image")
    pil_image.open = object
    pil.Image = pil_image
    for name, mod in (("cv2", cv2), ("PIL", pil), ("PIL.Image", pil_image)):
        sys.modules.setdefault(name, mod)


def main() -> None:
    if len(sys.argv) < 2:
        json.dump({"ok": False, "error": "missing url argument"}, sys.stdout, ensure_ascii=False)
        return
    url = sys.argv[1]
    try:
        _install_stubs()
        from parsehub import ParseHub

        result = ParseHub().parse_sync(url)
        data = result.to_dict()
        media = data.get("media") or {}
        video_url = media.get("url")
        if not video_url:
            json.dump(
                {"ok": False, "error": "未从页面解析到媒体直链"},
                sys.stdout,
                ensure_ascii=False,
            )
            return
        json.dump(
            {
                "ok": True,
                "platform": data.get("platform", ""),
                "title": data.get("title") or data.get("raw_url") or url,
                "name": data.get("name") or "",
                "video_url": video_url,
                "ext": media.get("ext"),
                "raw_url": data.get("raw_url"),
            },
            sys.stdout,
            ensure_ascii=False,
        )
    except Exception as e:  # noqa: BLE001 — sidecar 顶层兜底，错误统一走 JSON
        json.dump(
            {"ok": False, "error": f"{type(e).__name__}: {e}"},
            sys.stdout,
            ensure_ascii=False,
        )


if __name__ == "__main__":
    main()

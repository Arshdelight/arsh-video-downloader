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


def main() -> None:
    if len(sys.argv) < 2:
        json.dump({"ok": False, "error": "missing url argument"}, sys.stdout, ensure_ascii=False)
        return
    url = sys.argv[1]
    try:
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

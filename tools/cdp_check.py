"""Capture console errors from Castledom WASM page via Chrome CDP."""
from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import time
import urllib.request
from pathlib import Path

try:
    import websocket  # websocket-client
except ImportError:
    subprocess.check_call([sys.executable, "-m", "pip", "install", "websocket-client", "-q"])
    import websocket

CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"
PORT = 9344
URL = "http://127.0.0.1:8080/"


def main() -> None:
    user_data = tempfile.mkdtemp(prefix="castledom-cdp-")
    proc = subprocess.Popen(
        [
            CHROME,
            "--headless=new",
            "--disable-gpu",
            "--enable-webgl",
            "--use-angle=swiftshader",
            f"--remote-debugging-port={PORT}",
            "--remote-allow-origins=*",
            f"--user-data-dir={user_data}",
            URL,
        ],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    try:
        page = None
        for _ in range(30):
            time.sleep(0.5)
            try:
                with urllib.request.urlopen(f"http://127.0.0.1:{PORT}/json") as r:
                    tabs = json.load(r)
                page = next((t for t in tabs if URL.rstrip("/") in t.get("url", "")), None)
                if page:
                    break
            except Exception:
                pass
        if not page:
            print("NO_PAGE")
            return

        ws = websocket.create_connection(page["webSocketDebuggerUrl"], timeout=10)
        msg_id = 0
        logs: list[str] = []

        def send(method: str, params: dict | None = None) -> int:
            nonlocal msg_id
            msg_id += 1
            payload = {"id": msg_id, "method": method}
            if params is not None:
                payload["params"] = params
            ws.send(json.dumps(payload))
            return msg_id

        send("Runtime.enable")
        send("Log.enable")
        send("Console.enable")
        send("Page.enable")
        send("Page.reload", {"ignoreCache": True})

        deadline = time.time() + 25
        eval_id = None
        while time.time() < deadline:
            ws.settimeout(2)
            try:
                raw = ws.recv()
            except Exception:
                continue
            data = json.loads(raw)
            method = data.get("method", "")
            if method in (
                "Runtime.consoleAPICalled",
                "Runtime.exceptionThrown",
                "Log.entryAdded",
            ):
                logs.append(json.dumps(data, ensure_ascii=False)[:3000])
            if data.get("id") == eval_id and "result" in data:
                logs.append("EVAL " + json.dumps(data["result"], ensure_ascii=False)[:2000])
                break
            if time.time() > deadline - 5 and eval_id is None:
                eval_id = send(
                    "Runtime.evaluate",
                    {
                        "expression": """(() => {
  const c = document.getElementById('game-canvas');
  let webgl = null;
  try {
    const gl = c.getContext('webgl2');
    webgl = gl ? 'webgl2' : (c.getContext('webgl') ? 'webgl' : null);
  } catch (e) { webgl = String(e); }
  return {
    canvas: c ? {w:c.width,h:c.height,cw:c.clientWidth,ch:c.clientHeight} : null,
    webgl,
    bodyText: (document.body && document.body.innerText || '').slice(0,200),
  };
})()""",
                        "returnByValue": True,
                        "awaitPromise": False,
                    },
                )

        out = Path(__file__).resolve().parents[1] / "cdp-log.txt"
        out.write_text("\n----\n".join(logs) if logs else "NO_LOGS", encoding="utf-8")
        print(f"wrote {out} ({len(logs)} entries)")
        for line in logs[:12]:
            print(line[:500])
            print("====")
    finally:
        proc.kill()


if __name__ == "__main__":
    main()

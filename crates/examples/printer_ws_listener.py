"""
Simple Python PoC to print messages from the printer WebSocket feed.
Requires: `websocket-client` (pip install websocket-client)

Usage:
  set PRINTER_WS=ws://192.168.12.182:9999/
  set PRINTER_ORIGIN=http://192.168.12.182
  python crates\examples\printer_ws_listener.py

On PowerShell (Windows):
  $env:PRINTER_WS = 'ws://192.168.12.182:9999/' ; $env:PRINTER_ORIGIN = 'http://192.168.12.182' ; python .\crates\examples\printer_ws_listener.py
"""
import os
import websocket
import threading

WS_URL = os.environ.get("PRINTER_WS", "ws://192.168.12.182:9999/")
ORIGIN = os.environ.get("PRINTER_ORIGIN", "http://192.168.12.182")

def on_message(ws, message):
    print("MSG:", message)

def on_error(ws, error):
    print("ERROR:", error)

def on_close(ws, close_status_code, close_msg):
    print("CLOSED", close_status_code, close_msg)

def on_open(ws):
    print("CONNECTED to", WS_URL)

if __name__ == '__main__':
    headers = [f"Origin: {ORIGIN}"]
    ws = websocket.WebSocketApp(WS_URL,
                                header=headers,
                                on_open=on_open,
                                on_message=on_message,
                                on_error=on_error,
                                on_close=on_close)

    # run forever
    wst = threading.Thread(target=ws.run_forever, kwargs={"ping_interval": 20})
    wst.daemon = True
    wst.start()

    try:
        while True:
            pass
    except KeyboardInterrupt:
        print("Interrupted, exiting")
        ws.close()

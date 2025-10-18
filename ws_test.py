#!/usr/bin/env python3
import asyncio
import json
import websockets

WS_URL = "ws://127.0.0.1:8080/ws/infer"  # ✅ fixed path

async def test_unified_ws():
    print(f"🔌 Connecting to {WS_URL}...")
    async with websockets.connect(WS_URL) as ws:
        payload = { "text": "Explain TCP vs UDP", "mode": "sentiment" }
        print("📤 Sending:", payload)
        await ws.send(json.dumps(payload))

        reply = await ws.recv()
        print("📥 Raw response:", reply)

        try:
            data = json.loads(reply)
            print("✅ Parsed response:")
            print(json.dumps(data, indent=2))
        except json.JSONDecodeError:
            print("⚠️ Response is not valid JSON")

if __name__ == "__main__":
    asyncio.run(test_unified_ws())

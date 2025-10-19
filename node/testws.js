#!/usr/bin/env node
import WebSocket from "ws";

// Replace with your actual public Cloudflare WSS URL
const WS_URL = "wss://realize-housing-tub-html.trycloudflare.com";

const ws = new WebSocket(WS_URL);

ws.on("open", () => {
  console.log("✅ Connected to", WS_URL);

  // Send a test message
  const req = {
    id: "test1",
    text: "Which 3 user stories require developer attention?",
  };
  ws.send(JSON.stringify(req));
  console.log("📤 Sent:", req);
});

ws.on("message", (data) => {
  try {
    console.log("📥 Received:", data.toString());
  } catch {
    console.log("📥 Raw:", data);
  }
});

ws.on("close", () => {
  console.log("❌ Connection closed");
});

ws.on("error", (err) => {
  console.error("⚠️  Error:", err.message);
});

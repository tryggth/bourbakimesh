#!/usr/bin/env bash
set -euo pipefail

mkdir -p logs/

echo "======================================================================"
echo "🌟 BOURBAKIMESH INTERACTIVE WEB UI & TELEMETRY LAUNCHER"
echo "======================================================================"

# Clean up any previously running servers on ports 8000 or 5173
echo "🧹 Cleaning up existing server instances..."
fuser -k 8000/tcp 2>/dev/null || true
fuser -k 5173/tcp 2>/dev/null || true
pkill -f "bourbakimesh.api.demo_runner" 2>/dev/null || true

# 1. Start FastAPI Telemetry Backend on http://localhost:8000
echo "[1/3] Starting FastAPI Telemetry Backend on http://0.0.0.0:8000..."
nohup .venv/bin/python -m bourbakimesh.api.server \
  --host 0.0.0.0 \
  --port 8000 \
  --model-path checkpoints/bourbaki_v2.pt \
  > logs/api_server.log 2>&1 &
API_PID=$!
disown $API_PID 2>/dev/null || true

# Wait for backend to be ready
echo "⏳ Waiting for API server readiness..."
for i in {1..20}; do
  if curl -s http://127.0.0.1:8000/api/status >/dev/null 2>&1; then
    echo "✅ API server is online (PID: ${API_PID})!"
    break
  fi
  sleep 0.5
done

# 2. Start Vite Frontend on http://localhost:5173
echo "[2/3] Starting Vite Frontend on http://0.0.0.0:5173..."
nohup bash -c "cd ui && npm run dev -- --host 0.0.0.0 --port 5173" > logs/ui_dev.log 2>&1 &
UI_PID=$!
disown $UI_PID 2>/dev/null || true

# Wait for frontend to be ready
for i in {1..20}; do
  if curl -s http://127.0.0.1:5173 >/dev/null 2>&1; then
    echo "✅ UI frontend is online (PID: ${UI_PID})!"
    break
  fi
  sleep 0.5
done

# 3. Start Live Telemetry & Dialogue Feeder in background
echo "[3/3] Launching live theorem-proving telemetry feeder..."
nohup .venv/bin/python -m bourbakimesh.api.demo_runner \
  --host 127.0.0.1 \
  --port 8000 \
  --delay 0.8 \
  --loop \
  > logs/demo_feeder.log 2>&1 &
FEEDER_PID=$!
disown $FEEDER_PID 2>/dev/null || true

echo "======================================================================"
echo "🎉 BOURBAKIMESH LIVE DEMO RUNNING SUCCESSFULLY"
echo "======================================================================"
echo "🌐 Frontend Dashboard: http://localhost:5173"
echo "📡 Backend Telemetry API: http://localhost:8000/api/status"
echo "🔌 WebSocket Telemetry Stream: ws://localhost:8000/ws/telemetry"
echo ""
echo "Processes:"
echo "  - API Backend (PID: ${API_PID})"
echo "  - UI Frontend (PID: ${UI_PID})"
echo "  - Telemetry Feeder (PID: ${FEEDER_PID})"
echo ""
echo "Logs:"
echo "  - logs/api_server.log"
echo "  - logs/ui_dev.log"
echo "  - logs/demo_feeder.log"
echo "======================================================================"

#!/bin/bash
# Boots the RapidRAW generative-replace stack:
#   ComfyUI on 5545 (the port the connector expects) + connector on 5001
#   (5000 is squatted by macOS AirPlay Receiver / ControlCenter).
# RapidRAW settings → Generative AI → AI Connector → 127.0.0.1:5001
#
# Stops itself after IDLE_MIN minutes with no generation: once warm the stack
# holds ~9GB of SDXL weights, which exhausts a 16GB machine if left running.
# Ctrl-C stops both immediately. Logs: /tmp/comfyui.log, /tmp/ai-connector.log
set -e
IDLE_MIN=${IDLE_MIN:-15}
CONNECTOR_LOG=/tmp/ai-connector.log

cd "$HOME/ComfyUI"
# --input-directory: newer ComfyUI rejects LoadImage paths outside its input
# dir (traversal guard); the connector passes absolute paths from its cache.
./venv/bin/python main.py --port 5545 \
  --input-directory "$HOME/RapidRAW-AI-Connector/cache" >/tmp/comfyui.log 2>&1 &
COMFY_PID=$!
trap 'kill $COMFY_PID $CONNECTOR_PID $TAIL_PID 2>/dev/null' EXIT
echo "ComfyUI starting (pid $COMFY_PID, log /tmp/comfyui.log)..."
until curl -sf http://127.0.0.1:5545/system_stats >/dev/null 2>&1; do
  kill -0 $COMFY_PID 2>/dev/null || { echo "ComfyUI died — see /tmp/comfyui.log"; exit 1; }
  sleep 1
done
echo "ComfyUI up on 5545."

cd "$HOME/RapidRAW-AI-Connector"
: > "$CONNECTOR_LOG"
PORT=5001 ./venv/bin/python main.py > "$CONNECTOR_LOG" 2>&1 &
CONNECTOR_PID=$!
echo "Connector on 5001. Auto-stops after ${IDLE_MIN}m idle; Ctrl-C to stop now."

# Idle watchdog. RapidRAW health-polls /health every 10s, so the log never goes
# stale on its own — count /inpaint requests instead and stop when that stalls.
(
  seen=""; idle=0
  while sleep 60; do
    kill -0 $CONNECTOR_PID 2>/dev/null || exit 0
    n=$(grep -c "/inpaint" "$CONNECTOR_LOG" 2>/dev/null || true)
    if [ "$n" = "$seen" ]; then idle=$((idle + 1)); else seen=$n; idle=0; fi
    if [ "$idle" -ge "$IDLE_MIN" ]; then
      echo "[watchdog] no generation for ${IDLE_MIN}m — stopping stack"
      kill $CONNECTOR_PID 2>/dev/null
      exit 0
    fi
  done
) &

tail -f "$CONNECTOR_LOG" &
TAIL_PID=$!
wait $CONNECTOR_PID

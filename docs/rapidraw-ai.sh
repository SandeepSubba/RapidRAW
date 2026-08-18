#!/bin/bash
# Bridges RapidRAW to a ComfyUI backend. RapidRAW cannot talk to ComfyUI
# directly — it speaks this connector's /health + /inpaint protocol:
#
#     RapidRAW  ->  connector :5001  ->  ComfyUI
#
# Usage:
#   ~/rapidraw-ai.sh              pick from a menu
#   ~/rapidraw-ai.sh desktop      use Comfy Desktop (you start it; port 8188)
#   ~/rapidraw-ai.sh bundled      use the CLI install at ~/ComfyUI (auto-started)
#
# RapidRAW settings → Generative AI → AI Connector → 127.0.0.1:5001 (both modes)
# Ctrl-C stops everything this script started. Log: /tmp/ai-connector.log
set -e

PORT="${PORT:-5001}"          # 5000 is squatted by macOS AirPlay Receiver
BUNDLED_PORT=5545             # what the connector defaults to for the CLI stack
DESKTOP_PORT="${DESKTOP_PORT:-8188}"
MODE="$1"

if [ -z "$MODE" ]; then
  echo "Which ComfyUI should RapidRAW use?"
  echo "  1) Comfy Desktop   — your standalone app (start it yourself first)"
  echo "  2) Bundled CLI     — ~/ComfyUI, started and stopped by this script"
  printf "Choice [1]: "; read -r choice
  case "$choice" in 2) MODE=bundled ;; *) MODE=desktop ;; esac
fi

# A stale connector holding 5001 silently keeps serving its OLD target, which
# looks exactly like "the new settings did nothing". Always clear it first.
if lsof -ti :"$PORT" >/dev/null 2>&1; then
  echo "Port $PORT was busy (old connector) — clearing it."
  lsof -ti :"$PORT" | xargs kill -9 2>/dev/null || true
  sleep 1
fi

COMFY_PID=""
case "$MODE" in
  desktop)
    COMFY_PORT="$DESKTOP_PORT"
    if ! curl -sf -m 5 "http://127.0.0.1:$COMFY_PORT/system_stats" >/dev/null; then
      echo "Comfy Desktop is not answering on $COMFY_PORT — start the app first."
      exit 1
    fi
    # Comfy Desktop keeps input/ outside its install dir, and ComfyUI rejects
    # LoadImage paths outside it. Read the real path off the running process.
    DESKTOP_INPUT=$(ps -axo command= | awk '/ComfyUI\/main.py/ {for(i=1;i<=NF;i++) if($i=="--input-directory") {print $(i+1); exit}}')
    CACHE_DIR="${CACHE_DIR:-${DESKTOP_INPUT:-$HOME/ComfyUI-Shared/input}/rapidraw}"
    echo "Using Comfy Desktop on $COMFY_PORT."
    ;;
  bundled)
    COMFY_PORT="$BUNDLED_PORT"
    CACHE_DIR="${CACHE_DIR:-$HOME/ComfyUI/input/rapidraw}"
    mkdir -p "$CACHE_DIR"
    if curl -sf -m 5 "http://127.0.0.1:$COMFY_PORT/system_stats" >/dev/null; then
      echo "Bundled ComfyUI already up on $COMFY_PORT."
    else
      echo "Starting bundled ComfyUI on $COMFY_PORT (log /tmp/comfyui.log)..."
      ( cd "$HOME/ComfyUI" && ./venv/bin/python main.py --port "$COMFY_PORT" \
          --input-directory "$HOME/ComfyUI/input" >/tmp/comfyui.log 2>&1 ) &
      COMFY_PID=$!
      until curl -sf -m 5 "http://127.0.0.1:$COMFY_PORT/system_stats" >/dev/null 2>&1; do
        kill -0 $COMFY_PID 2>/dev/null || { echo "ComfyUI died — see /tmp/comfyui.log"; exit 1; }
        sleep 1
      done
      echo "Bundled ComfyUI up."
    fi
    ;;
  *) echo "Unknown mode '$MODE' (use: desktop | bundled)"; exit 1 ;;
esac

mkdir -p "$CACHE_DIR"
trap 'kill $COMFY_PID 2>/dev/null' EXIT

echo "Connector on $PORT  ->  ComfyUI on $COMFY_PORT"
echo "Cache: $CACHE_DIR"
cd "$HOME/RapidRAW-AI-Connector"
COMFY_PORT="$COMFY_PORT" PORT="$PORT" CACHE_DIR="$CACHE_DIR" \
  ./venv/bin/python main.py 2>&1 | tee /tmp/ai-connector.log

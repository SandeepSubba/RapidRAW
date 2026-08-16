# Generative replace: local ComfyUI stack

The "Generative AI" settings section powers exactly one feature — prompted
generative replace. All other AI (masks, tagging, denoise, Quick Erase/LaMa)
is local ONNX and needs no backend. The app does not speak the ComfyUI API;
it needs the companion middleware
[RapidRAW-AI-Connector](https://github.com/CyberTimon/RapidRAW-AI-Connector)
(`GET /health`, `POST /upload_source`, `POST /inpaint`).

## Port traps

- The connector defaults to **5000**, but macOS AirPlay Receiver
  (ControlCenter) squats that port — we run it with `PORT=5001` (pydantic
  BaseSettings reads env). RapidRAW gets `127.0.0.1:5001`
  (Settings → Generative AI → AI Connector). The `8188` placeholder in the
  settings UI is misleading.
- The connector expects **ComfyUI on 5545** (hardcoded in its `engine.py`),
  not ComfyUI's default 8188 — ComfyUI must be started with `--port 5545`.
- ComfyUI must also get
  `--input-directory ~/RapidRAW-AI-Connector/cache` — newer ComfyUI rejects
  LoadImage paths outside its input dir (traversal guard), and the connector
  passes absolute paths from its cache.
- The in-app **Test** button is only a reachability check (any HTTP response
  passes, even a 404) — green does not prove the protocol works.

## Home-Mac install (M1 Pro, done 2026-08-16)

- `~/ComfyUI` — clone, venv on Homebrew python@3.12 (3.14 has no torch
  wheels), torch MPS + requirements.
- `~/ComfyUI/custom_nodes/ComfyUI-Inpaint-CropAndStitch` — provides the
  `InpaintCropImproved`/`InpaintStitchImproved` nodes the connector's
  `workflow.json` uses.
- Models (filenames must match `workflow.json` exactly):
  - `models/checkpoints/XL_RealVisXL_V5.0_Lightning.safetensors`
    (RealVisXL V5.0 Lightning fp16, ~6.5GB, HF `SG161222`)
  - `models/vae/sdxl_vae.safetensors` (HF `stabilityai/sdxl-vae`)
  - `models/controlnet/diffusion_pytorch_model_promax.safetensors`
    (HF `xinsir/controlnet-union-sdxl-1.0`, ~2.5GB)
- `~/RapidRAW-AI-Connector` — clone, venv (same python), requirements.
- `~/start-ai.sh` — boots ComfyUI on 5545, waits for it, then the connector
  on 5001. Ctrl-C stops both. Logs: `/tmp/comfyui.log`,
  `/tmp/ai-connector.log`.

## Start/stop is automatic

The stack is never meant to sit idle: warm ComfyUI holds many GB of SDXL
weights on unified memory and will exhaust a 16GB machine (it did, once).

- **Stops itself.** `start-ai.sh` runs an idle watchdog and shuts both
  processes down after `IDLE_MIN` (default 15) minutes with no generation.
  It counts `/inpaint` requests in the connector log, not log activity —
  RapidRAW health-polls every 10s, so any mtime-based timer would never fire.
  Override per run: `IDLE_MIN=5 ~/start-ai.sh`.
- **Starts on demand.** `ai_connector::ensure_local_stack` (called from the
  `ai-connector` branch of `invoke_generative_replace_with_mask_def`) spawns
  `~/start-ai.sh` when the connector is down, then polls `/health` for up to
  3 minutes. Gated on a loopback address and the script existing, so a remote
  connector is never second-guessed.
- The status poll emits `canStart` alongside `connected`, and `AIPanel`'s
  `isGenerativeAvailable` ORs it in — otherwise the prompt field is disabled
  while the stack is down and nothing could ever trigger the start.

First edit after an idle stop costs ~3 min (boot + weight load); later ones
are fast until it goes idle again.

Verified end-to-end 2026-08-16: `/upload_source` + `/inpaint` ("a red
flower" on a synthetic scene) → correct composited crop in 155s including
first-load of the models. Expect ~1–3 min per generation on M1 Pro; Quick
Erase (LaMa) remains the right tool for plain dust/object removal.

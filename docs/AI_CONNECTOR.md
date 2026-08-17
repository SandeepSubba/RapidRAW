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

## RapidRAW does not manage ComfyUI

The app never starts, stops, or supervises a ComfyUI stack — it only talks to
whatever `aiConnectorAddress` points at. Run ComfyUI yourself (the standalone
Comfy Desktop app on this Mac), keep the connector middleware pointed at it,
and set that connector's address in Settings.

The CLI stack below (`~/ComfyUI` + `docs/start-ai.sh`) is an optional
alternative to the standalone app, not something the app depends on. Its
watchdog matters if you use it: warm ComfyUI holds many GB of SDXL weights on
unified memory and will exhaust a 16GB machine if left idle.

<details><summary>Optional CLI stack (start-ai.sh)</summary>

The stack is never meant to sit idle: warm ComfyUI holds many GB of SDXL
weights on unified memory and will exhaust a 16GB machine (it did, once).

- **Stops itself.** `start-ai.sh` runs an idle watchdog and shuts both
  processes down after `IDLE_MIN` (default 15) minutes with no generation.
  It counts `/inpaint` requests in the connector log, not log activity —
  RapidRAW health-polls every 10s, so any mtime-based timer would never fire.
  Override per run: `IDLE_MIN=5 ~/start-ai.sh`.
</details>

Generative controls stay disabled until the connector answers `/health`, so
start your ComfyUI and connector before reaching for generative replace.

Verified end-to-end 2026-08-16: `/upload_source` + `/inpaint` ("a red
flower" on a synthetic scene) → correct composited crop in 155s including
first-load of the models. Expect ~1–3 min per generation on M1 Pro; Quick
Erase (LaMa) remains the right tool for plain dust/object removal.

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
- `~/rapidraw-ai.sh` (copy in `docs/`) — the launcher, see below.
  Logs: `/tmp/comfyui.log`, `/tmp/ai-connector.log`.

## RapidRAW does not manage ComfyUI

The app never starts, stops, or supervises ComfyUI — it only talks to whatever
`aiConnectorAddress` points at, which must be the **connector**, never ComfyUI
itself (RapidRAW speaks `/health` + `/inpaint`, not the ComfyUI API).

## Switching backends: `~/rapidraw-ai.sh`

One launcher, two backends, same address in RapidRAW (`127.0.0.1:5001`):

    ~/rapidraw-ai.sh            # menu
    ~/rapidraw-ai.sh desktop    # Comfy Desktop on 8188 (start the app yourself)
    ~/rapidraw-ai.sh bundled    # CLI install at ~/ComfyUI, started/stopped for you

Both modes verified 2026-08-18. Details it handles so you don't have to:

- **Frees port 5001 first.** A stale connector keeps serving its OLD target, so
  a switch looks like it did nothing. This is the single most confusing failure
  mode — it silently wasted a debugging round.
- **Finds Comfy Desktop's input dir** by reading `--input-directory` off the
  running process (it lives at `~/ComfyUI-Shared/input`, *not* in the install
  folder). ComfyUI rejects LoadImage paths outside that dir, so the connector's
  cache is placed inside it.
- **Bundled mode** launches `~/ComfyUI` on 5545 with a matching
  `--input-directory`, and stops it on Ctrl-C.

Models are shared: the Desktop install's `models/` and `custom_nodes/` are
symlinks into `~/ComfyUI`, so both backends see the same ~9GB without a second
copy. The workflow needs the Inpaint-CropAndStitch nodes, which **only load at
ComfyUI startup** — restart ComfyUI after linking anything new.

Generative controls stay disabled until the connector answers `/health`, so
start your ComfyUI and connector before reaching for generative replace.

Verified end-to-end 2026-08-16: `/upload_source` + `/inpaint` ("a red
flower" on a synthetic scene) → correct composited crop in 155s including
first-load of the models. Expect ~1–3 min per generation on M1 Pro; Quick
Erase (LaMa) remains the right tool for plain dust/object removal.

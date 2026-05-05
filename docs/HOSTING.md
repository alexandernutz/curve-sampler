# Hosting Notes

## What needs to be served
- `index.html` (already in repo)
- `pkg/` directory — wasm-pack build output (currently gitignored)

## Two deployment approaches

### Option A — commit `pkg/` (simplest, no CI)
Remove `pkg` from `.gitignore`, run:
```bash
wasm-pack build --target web --release
```
Commit the result. Pages serves the repo root directly.
Whenever you update the code, rebuild and recommit `pkg/`.
Binary artifacts in git isn't ideal but fine for a small WASM file.

### Option B — CI pipeline (cleaner)
A small workflow file that runs on push to `main`:
install wasm-pack → build → push `index.html` + `pkg/` to a deploy branch.
Never touch build artifacts manually.

The workflow is almost identical across all three platforms:

| Platform  | CI file                          | Pages output       | Notes |
|-----------|----------------------------------|--------------------|-------|
| GitHub    | `.github/workflows/deploy.yml`   | `gh-pages` branch  | Marketplace has ready-made wasm-pack actions |
| GitLab    | `.gitlab-ci.yml`                 | `public/` directory| Built-in Pages support |
| Codeberg  | `.forgejo/workflows/deploy.yml`  | `pages` branch     | Forgejo Actions, GitHub-compatible syntax |

## HTTPS
All three provide HTTPS automatically on their default domains
(`*.github.io` / `*.gitlab.io` / `*.codeberg.page`) — required for WASM,
no configuration needed.

## Recommendation
Start with Option A to get it live with zero friction, switch to CI later
for automated deploys.

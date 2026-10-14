# 002 The three.js engine, its demo and bench pages removed

status: merged · rank: 2 · level: simple · type: refactor · request: user, 2026-10-14 ("clean lancien code") · light path · after: []

## Summary
Rank: difficulty 1 (five files deleted, two edited, one dependency), impact 2 (the fallback when the Rust engine cannot start goes; three.js needed WebGL too) → 2.
The Rust engine (wgpu: WebGPU, else WebGL2) draws the map; three.js (`src/map/engine.ts`) was only its fallback (`?engine=three` or a failed start). Removed with `rs.html`, `bench.html`, `src/rs-demo.ts`, `src/engine-bench.ts` and `three`, `@types/three`; `EngineEvents` moved to `src/map/rust-engine.ts`. React, Vite, ngraph and the Node scripts stay until 008 (removing them before the Leptos app exists would break the site).

## Log
| Step | Result | Output |
| --- | --- | --- |
| Build | done | tsc 0, vite build 451.6 kB (140.3 kB gzip) |
| Merge | done | PR #2 squash 27ecad9, CI 10/10 green |

## Retro
One line (light path): opened without its task file — written after the merge; a change of any size gets its `.task` file before its first commit.

# Third-party notices

## codebase-memory-mcp — graph-ui render layer

The Rationale Control Room (`ui/`, served by `rationale ui`) adapts parts of
the Three.js render layer of
[codebase-memory-mcp](https://github.com/DeusData/codebase-memory-mcp)
(`graph-ui/`, reference commit `7d74337`). Only renderer techniques were
adapted; Rationale does not include Codebase Memory's application, backend
contract, or any other part of its code. Codebase Memory remains an external
structural provider reached through its public MCP tools.

| Rationale file | Adapted from |
|---|---|
| `ui/src/graph/NodeCloud.tsx` | `graph-ui/src/components/NodeCloud.tsx` |
| `ui/src/graph/EdgeLines.tsx` | `graph-ui/src/components/EdgeLines.tsx` and the point sprite of `NodeCloud.tsx` |
| `ui/src/graph/NodeLabels.tsx` | `graph-ui/src/components/NodeLabels.tsx` |
| `ui/src/graph/Stage.tsx` | `graph-ui/src/components/GraphScene.tsx` |
| `ui/src/graph/palette.ts` | `graph-ui/src/components/EdgeLines.tsx`, `graph-ui/src/lib/colors.ts` |

Each adapted file keeps a header naming its origin and this license.

```text
MIT License

Copyright (c) 2025 DeusData

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## Control Room bundle dependencies

When `ui/dist` is built, the embedded bundle contains code from these npm
packages (exact versions pinned in `ui/package-lock.json`); each ships its
license in its package:

| Package | License |
|---|---|
| `react`, `react-dom` | MIT |
| `three` | MIT |
| `@react-three/fiber` | MIT |
| `d3-force-3d` | MIT |

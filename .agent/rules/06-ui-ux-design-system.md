# Rule 06: UI/UX Pro Max & Git Primer Design System
**Scope:** Desktop Frontend (`ui/src/**`), Themes, Icons, Accessibility, Layout & Motion  
**Reference Skill:** `ui-ux-pro-max` (Accessibility, Touch & Interaction, Performance, Layout, Typography & Color, Animation, Style Consistency)  
**Severity:** Mandatory  

---

## 1. Visual Identity & Core Aesthetic

GitUpload Desktop adopts an **Authentic GitHub Primer & Git Engineering Suite** aesthetic combining high-density developer ergonomics with calm, accessible contrast across multiple color themes.

1. **Zero Emoji Icons (Mandatory)**:
   - NEVER use Unicode emojis (`🚀`, `📁`, `⚙️`, `🔥`, `✓`, `✗`, `⚠`) as UI icons, tab markers, button prefixes, or status badges.
   - Always import vector SVG icons from the approved icon libraries:
     - **Primary (Git & GitHub domain)**: `@primer/octicons-react` (`MarkGithubIcon`, `GitBranchIcon`, `GitCommitIcon`, `GitPullRequestIcon`, `GitMergeIcon`, `RepoIcon`, `ShieldCheckIcon`, `PlayIcon`, `TagIcon`, `IssueOpenedIcon`, `TerminalIcon`, `SparklesFillIcon`).
     - **Secondary (General UI utilities & theme controls)**: `lucide-react` (`Palette`, `FolderOpen`, `ArrowRight`, `Check`, `X`, `AlertTriangle`, `RefreshCw`, `SlidersHorizontal`).
   - Keep inline icon sizing consistent (`14px` / `16px` inside buttons & menus, `18px` / `20px` in section headers, `28px+` in empty states).

2. **Typography & Developer Hierarchy**:
   - **UI Sans Stack**: `-apple-system, BlinkMacSystemFont, "Segoe UI", "IBM Plex Sans", "Noto Sans", Helvetica, Arial, sans-serif` (local system stack; never fetch external CDN fonts so strict offline `CSP` remains intact).
   - **Code & Data Mono Stack**: `"JetBrains Mono", "Cascadia Mono", "Fira Code", Consolas, monospace` with `font-variant-numeric: tabular-nums` on SHAs, file sizes, line counts, percentages, and rate limits.
   - **Readability**: Body line-height `1.5`, compact control text `13px`, labels `12px–13px` semibold, headings clearly tiered (`20px` view title, `15px` card header).

---

## 2. Git-Harmonized Multi-Theme Token Architecture

All surfaces, text, borders, status indicators, and Canvas DAG strokes MUST consume CSS custom properties (`var(--color-*)`) rather than hardcoded hex colors. Every theme MUST satisfy **WCAG AA (`>= 4.5:1`)** text contrast:

| Theme ID | Display Name | Mode | Canvas (`--color-canvas-default`) | Surface (`--color-canvas-subtle`) | Primary Accent (`--color-accent-fg`) | Git Highlight |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `dark` | GitHub Dark Default | Dark | `#0d1117` | `#161b22` | `#58a6ff` | `#3fb950` (Git Green) |
| `dimmed` | GitHub Dark Dimmed | Dark | `#22272e` | `#2d333b` | `#539bf5` | `#57ab5a` (Soft Green) |
| `git_aurora` | Git Copper Obsidian | Dark | `#0b0f14` | `#131922` | `#f05032` (Git Brand) | `#38bdf8` (Branch Cyan) |
| `high_contrast` | GitHub High Contrast | Dark (AAA) | `#010409` | `#0a0c10` | `#71b7ff` | `#26cd4d` (Vivid Green) |
| `light` | GitHub Light Default | Light | `#ffffff` | `#f6f8fa` | `#0969da` | `#1a7f37` (Forest Green) |
| `warm_paper` | Git Soft Sepia | Light | `#faf8f5` | `#f2efe9` | `#c2410c` (Copper Ink) | `#15803d` (Sage Green) |

- When switching themes, both `.app-root.theme-<id>` class AND `document.documentElement.dataset.colorMode` (`"dark"` or `"light"`) MUST update synchronously so `@primer/css` built-in controls (`.btn`, `.Label`, `.flash`) adapt seamlessly.
- In HTML5 Canvas components (`GitGraphView`), read resolved theme colors via `getComputedStyle(...)` so commit node rings and swimlanes always match active light/dark polarity.

---

## 3. Accessibility & Interaction Rules (`ui-ux-pro-max` P1–P2)

1. **Keyboard Focus & ARIA**:
   - Every interactive control (`button`, `input`, `select`, clickable row) MUST render a high-visibility `:focus-visible` ring (`2px solid var(--color-accent-fg)`, `outline-offset: 2px`).
   - Every icon-only button (e.g., copy, open external URL, delete repo, close drawer) MUST define both `aria-label` and `title`.
   - Navigation buttons MUST expose `aria-current="page"` when selected.
2. **Targets & Feedback**:
   - Sidebar & toolbar action buttons keep comfortable clickable height (`>= 34px–40px`) and `cursor: pointer` on all interactive elements.
   - Async operations MUST disable trigger controls and surface explicit inline progress or error banners (`role="alert"`).
3. **Zero Layout-Shift Hover & Reduced Motion**:
   - Transitions (`150ms ease`) are restricted to `background-color`, `border-color`, `color`, `opacity`, and `box-shadow`—never apply `transform: scale()` on cards/buttons.
   - Wrap transitions/animations under `@media (prefers-reduced-motion: reduce)` fallbacks.

---

## 4. Layout Structure & Empty States

- **Desktop App Shell**:
  - Top Global Bar (`52px` fixed height): Brand lockup + Workspace repo chip + Vault health pill + Theme Selector dropdown.
  - Left Navigation Rail (`228px` width, collapsible/scrollable): Categorized groups (*Git Workflow*, *GitHub Suite*, *System*) with left accent indicator on active item.
  - Fluid Workspace Canvas: Structured `primer-card` elevation, `16px–20px` section spacing, crisp empty-state cards with descriptive icon + next action guidance instead of blank lists.
  - Floating Cloud AI Drawer (`z-index: 50`): Properly spaced from viewport edges (`20px` margin), clear borders in both dark and light modes.

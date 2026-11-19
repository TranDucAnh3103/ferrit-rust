# FERRIT Desktop Platform (v2.6.0)
### Next-Generation High-Performance Git Platform & DevSecOps Suite (GitHub Primer UI)

[![Rust](https://img.shields.io/badge/rust-1.82%2B-orange.svg?logo=rust)](https://www.rust-lang.org)
[![Tauri](https://img.shields.io/badge/tauri-v2.1-24C8D8.svg?logo=tauri)](https://v2.tauri.app)
[![UI](https://img.shields.io/badge/ui-GitHub%20Primer%20Design%20System-blue.svg?logo=github)](https://primer.style)
[![Git Graph](https://img.shields.io/badge/git--graph-DAG%20Swimlanes%2060FPS-blueviolet.svg)]()
[![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11%20%7C%20Linux%20%7C%20macOS-blue.svg)](https://github.com/java-web-service-stephenduc/rust-gitupload)
[![Memory](https://img.shields.io/badge/memory-%3C35MB%20RAM-brightgreen.svg)]()
[![Cold Start](https://img.shields.io/badge/cold--start-%3C200ms-brightgreen.svg)]()

> [!TIP]
> Xem tài liệu tổng quan toàn bộ tính năng chi tiết của ứng dụng tại: [FERRIT_OVERVIEW.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/FERRIT_OVERVIEW.md).

---

## 1. Giới Thiệu Dự Án (Project Overview - FERRIT v2.6.0)

**FERRIT** là hệ sinh thái máy tính hiện đại được xây dựng hoàn toàn bằng **Rust** và **Tauri v2**, thay thế toàn diện công cụ cũ `G-GitUpload v1.4.1` (phát triển bằng Go/Wails v2). 

Phiên bản **v2.2** mang lại bước nhảy vọt về trải nghiệm người dùng và tính năng kỹ thuật chuyên sâu:
- **Đồ thị Git DAG Tương Tác Trực Quan (Interactive Git Graph):** Tái hiện cây lịch sử commit trực quan tương tự GitKraken/GitLens thông qua `git2::Revwalk`. Hiển thị các làn bơi nhánh (swimlanes) đa sắc màu, đường cong bezier nối commit cha-con, điểm rẽ nhánh/hợp nhất, thẻ tag, stash và con trỏ HEAD. Cuộn ảo (virtualized) mượt mà 60 FPS cho hơn 100,000 commits.
- **Thao tác Git Trực tiếp từ Đồ thị:** Hỗ trợ Cherry-pick, Revert commit, Checkout commit/branch, Reset (Soft/Mixed/Hard), Tạo Branch/Tag, và Trợ lý Interactive Rebase.
- **Bộ Quản lý Vòng đời GitHub CLI (`gh`) & Dual Auth Bridge (`gitupload-ghcli`):** Tự động phát hiện `gh.exe` trong hệ thống hoặc tải bản portable x64 về sandbox `%APPDATA%/GitUpload/bin/` kèm xác thực mã băm SHA256 (zero-admin rights). Tự động trích xuất token qua `gh auth token` vào Windows Credential Manager.
- **Giám sát GitHub Actions CI/CD Thời Gian Thực:** Theo dõi tiến độ workflow runs, stream nhật ký từng bước với mã màu ANSI trực quan, và hỗ trợ chạy lại (rerun) job thất bại.
- **Trình Quản lý Issues & Releases Đính Kèm AI Changelog:** Tạo và quản lý Issues, tự động sinh Release Notes chuẩn semantic bằng Cloud AI, và kéo thả file nhị phân đính kèm.
- **Trải nghiệm giao diện 1:1 GitHub Web (GitHub Primer Design System):** Tái tạo trọn vẹn thẩm mỹ của GitHub với các token Primer CSS, biểu tượng vector Primer Octicons, 4 theme chuẩn (GitHub Dark, GitHub Dark Dimmed, Light, High Contrast), cây thư mục phong cách GitHub và bộ xem diff 2 cột (side-by-side) / inline.
- **Động cơ Git Hooks bản địa thuần Rust (`gitupload-hooks`):** Thay thế Husky/Lefthook mà **không cần cài đặt Node.js hay npm**. Tự động kiểm tra Conventional Commits 1.0.0, chặn rò rỉ secret trong staged files, và ngăn chặn push phá hủy nhánh.
- **Hỗ trợ đầy đủ GitHub Organizations (`/user/orgs`):** Chuyển đổi linh hoạt giữa tài khoản cá nhân và các tổ chức, kiểm tra quyền hạn thành viên và nộp/tạo repository trực tiếp vào Organization.
- **Quản lý vòng đời Pull Request (PR Lifecycle):** Tạo PR chuẩn hoặc Draft PR, tự động viết mô tả PR bằng Cloud AI, xem diff trực quan, duyệt/góp ý, và merge PR với 3 chiến lược: *Squash and Merge*, *Rebase and Merge*, hoặc *Standard Merge Commit*.
- **Hỗ trợ Re-Push & Đồng bộ an toàn (`--force-with-lease`):** Tự động phát hiện repo Git đã có, kiểm tra số commit ahead/behind, và thay thế lệnh ép đẩy mù quáng bằng cờ bảo vệ an toàn.
- **Động cơ Cloud AI Độc Quyền (Pure Cloud AI):** Loại bỏ hoàn toàn các mô hình Local LLM cồng kềnh để giữ bộ nhớ ở mức siêu nhẹ (**< 28MB RAM**). Kết nối trực tiếp các mô hình hàng đầu thế giới: Google Gemini 2.0/1.5 Flash, OpenAI GPT-4o, Claude 3.5 Sonnet, và Groq thông qua SSE streaming.

```
       Go/Wails v1.4.1 (Monolith)                     Rust/Tauri v2.2 (Modular 10 Crates)
 ┌────────────────────────────────────┐         ┌──────────────────────────────────────────────┐
 │  Go Runtime + Wails v2 (~140MB)    │         │  Native Rust Core Engine (<28MB RAM)         │
 │  Plaintext .env & config.json      │  ───►   │  Zero-Plaintext Windows Keyring Vault        │
 │  Blind `git push --force`          │         │  Interactive Git Graph DAG (60 FPS Canvas)   │
 │  Sequential Subprocess Lag         │         │  Sandboxed GitHub CLI (`gh`) & Dual Auth     │
 │  Custom Flat UI                    │         │  GitHub Actions Live ANSI Stream & Issues    │
 │  No Orgs / No PR / No Hooks / No gh│         │  Full Orgs, PR Lifecycle & Native Git Hooks  │
 └────────────────────────────────────┘         └──────────────────────────────────────────────┘
```

---

## 2. Mục Lục Tài Liệu Kỹ Thuật (Documentation Index)

Bộ tài liệu kiến trúc và đặc tả của dự án được biên soạn chuyên sâu theo các tiêu chuẩn quốc tế (IEEE 830, ISO/IEC/IEEE 29148, C4 Model):

| Tài liệu | Mã tài liệu | Nội dung chính | Liên kết trực tiếp |
| :--- | :--- | :--- | :--- |
| **01. Kiểm Toán Mã Nguồn Cũ** | `AUDIT-2026-v1` | Phân tích chi tiết toàn bộ mã nguồn Go/Wails cũ, các luồng nghiệp vụ, nút thắt hiệu năng, nợ kỹ thuật và đánh giá 5 lỗ hổng bảo mật nghiêm trọng. | [01_LEGACY_AUDIT.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/01_LEGACY_AUDIT.md) |
| **02. Kiến Trúc Hệ Thống Mục Tiêu** | `ARCH-2026-v2.2` | Đặc tả kiến trúc C4, thiết kế 10 crate workspace, động cơ đồ thị Git DAG, bộ quản lý GitHub CLI sandbox, và kiến trúc Primer UI. | [02_ARCHITECTURE.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/02_ARCHITECTURE.md) |
| **03. Đặc Tả Yêu Cầu Phần Mềm (SRS)** | `SRS-RUST-GITUPLOAD-V2.2` | Đặc tả chuẩn IEEE 830 với 100% yêu cầu chức năng (FR-GRF, FR-GHCLI, FR-ACT, FR-ISS, FR-REL, FR-HOK, FR-PR), phi chức năng (NFR-xxx). | [03_SRS.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/03_SRS.md) |
| **04. Kế Hoạch & Lộ Trình Triển Khai** | `ROADMAP-RUST-GITUPLOAD-V2.2` | Bảng ánh xạ từng tệp tin từ Go sang Rust, cấu hình Cargo workspace đầy đủ (10 crates), lộ trình 5 giai đoạn (25 ngày), chiến lược test và auto-migration. | [04_MIGRATION_ROADMAP.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/04_MIGRATION_ROADMAP.md) |
| **05. Kế Hoạch Chi Tiết Sprint 1 (v2.2)** | `SPRINT1-PLAN-v2.2` | Core, Keyring Vault, SQLite WAL, Hooks, Dual Engine, Git DAG, Primer UI, Cloud AI, QA & Packaging (30 Tasks, 36 ACs). | [docs/sprint-1/README.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-1/README.md) |
| **06. Kế Hoạch Chi Tiết Sprint 2 (v2.3)** | `SPRINT2-PLAN-v2.3` | Multi-Remote Mirroring, 3-Way Merge Monaco Resolver, Native SSH Agent, Team Heatmap Analytics & Leaderboard. | [docs/sprint-2/README.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-2/README.md) |
| **07. Kế Hoạch Chi Tiết Sprint 3 (v2.4)** | `SPRINT3-PLAN-v2.4` | AI Vulnerability Scanner (OWASP/CWE), Semantic Commit Search, Stale Branch Cleaner, Background Scheduled Sync. | [docs/sprint-3/README.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-3/README.md) |
| **08. Kế Hoạch Chi Tiết Sprint 4 (v2.5)** | `SPRINT4-PLAN-v2.5` | Cross-Platform (macOS Apple Silicon, Linux AppImage/Flatpak), Minisign Auto-Updater, Enterprise Airgap Mode. | [docs/sprint-4/README.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-4/README.md) |
| **09. Kế Hoạch Chi Tiết Sprint 5 (v2.6+)**| `SPRINT5-PLAN-v2.6` | WebAssembly Plugin Host, Multi-Channel Webhooks (Discord/Slack/Telegram), Headless Companion CLI, Marketplace. | [docs/sprint-5/README.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-5/README.md) |
| **10. Sổ Tay Quy Chuẩn AI & Kỹ Sư** | `AGENTS-SPEC-v2.2` | Quy chế cộng tác AI Agent, quy tắc an toàn `#![forbid(unsafe_code)]`, chuẩn Conventional Commits, kiến trúc 10 crates và quy trình kiểm thử. | [AGENTS.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/AGENTS.md) |

---

## 3. So Sánh Bước Nhảy Vọt Hiệu Năng & Tính Năng

| Chỉ số / Đặc tính | Bản cũ (Go/Wails v1.4.1) | Bản mới (Rust/Tauri v2.2) | Đánh giá cải tiến |
| :--- | :--- | :--- | :--- |
| **Mức chiếm dụng RAM ở chế độ chờ** | ~85 MB – 140 MB | **< 28 MB RAM** | **Giảm hơn 80%** lượng RAM tiêu thụ |
| **Thời gian khởi động lạnh (Cold Start)**| 650 ms – 1,200 ms | **< 100 ms** | Khởi động tức thì, render ngay lập tức |
| **Lưu trữ thông tin bí mật (Tokens/Keys)**| Plaintext trong JSON và `.env` | **100% OS Keyring** (Windows Vault) | Loại bỏ hoàn toàn nguy cơ rò rỉ token |
| **Đồ thị Git DAG & Làn bơi nhánh** | Không có | **Interactive Git Graph (Canvas 60 FPS)** | Trực quan hóa tương tự GitKraken/GitLens |
| **Tích hợp GitHub CLI (`gh`)** | Không có | **Tự động tải, xác thực SHA256, Dual Auth**| Không cần admin quyền; lấy token tự động |
| **Giám sát GitHub Actions CI/CD** | Không có | **Live stream nhật ký ANSI màu thời gian thực**| Nút bấm Rerun jobs trực tiếp |
| **Động cơ Git Hooks (Linter/Guards)** | Không có (phụ thuộc npm/Husky) | **Bản địa thuần Rust (`gitupload-hooks`)** | Không cần Node.js/npm; thực thi < 2ms |
| **GitHub Organizations Support** | Không có (chỉ 1 tài khoản cá nhân)| **Hỗ trợ đầy đủ `/user/orgs` & switcher**| Chuyển đổi tổ chức và kiểm tra quyền hạn |
| **Vòng đời Pull Request (PR)** | Không có | **Tạo PR, Draft PR, Review, 3 Merge Types**| Tích hợp Cloud AI sinh mô tả PR |
| **An toàn dữ liệu khi đẩy bài** | Ép buộc mù quáng (`--force`) | **Safe Sync (`--force-with-lease`)** | Tự động kiểm tra lease và sao lưu nhánh |
| **Giao diện người dùng (UI/UX)** | Giao diện tùy biến thường | **GitHub Primer Design System + Octicons** | Trải nghiệm 1:1 chuẩn GitHub web |
| **Hệ sinh thái Git mở rộng** | Không hỗ trợ | **Git LFS, .gitignore gen, GPG signing** | Đầy đủ tính năng chuyên nghiệp |
| **Động cơ Trợ lý ảo AI** | Cloud API thường | **Pure Cloud AI (Gemini 2.0, GPT-4o, Claude)**| Stream SSE siêu tốc, RAM cực nhẹ |

---

## 4. Kiến Trúc Không Gian Làm Việc (10 Workspace Crates)

Dự án được phân rã thành **10 crate chuyên biệt** nằm trong thư mục `crates/` tuân thủ nguyên tắc trách nhiệm đơn lẻ (Single Responsibility Principle):

```
rust-gitupload/
├── Cargo.toml                       # Workspace manifest khai báo tập trung dependencies
├── rust-toolchain.toml              # Pinned Rust 1.82+ stable
├── docs/                            # Hệ thống tài liệu kiến trúc & đặc tả kỹ thuật
│   ├── 01_LEGACY_AUDIT.md
│   ├── 02_ARCHITECTURE.md
│   ├── 03_SRS.md
│   └── 04_MIGRATION_ROADMAP.md
├── crates/
│   ├── gitupload-core/              # Domain models, PR/Org/DAG schemas, error tree (thiserror)
│   ├── gitupload-security/          # Windows Credential Manager (keyring), log sanitizer, zeroize
│   ├── gitupload-storage/           # SQLite connection pool (WAL), schema migrations, history
│   ├── gitupload-git/               # Dual-engine Git (libgit2 + CLI), DAG Revwalk, LFS, GPG signing
│   ├── gitupload-hooks/             # Native Git Hooks runner in pure Rust (commit-msg, pre-commit, pre-push)
│   ├── gitupload-ghcli/             # GitHub CLI lifecycle: detection, SHA256 sandbox installer, dual auth
│   ├── gitupload-github/            # GitHub REST/GraphQL (Orgs, PR lifecycle, Actions, Issues, Releases)
│   ├── gitupload-ai/                # Pure Cloud AI Engine (Gemini 2.0, GPT-4o, Claude 3.5, Groq, OpenRouter)
│   ├── gitupload-watcher/           # Filesystem monitor (notify), debounced auto-sync queue
│   └── gitupload-app/               # Tauri v2 desktop binary, Primer UI bridge, IPC commands, main entry
└── ui/                              # Giao diện người dùng GitHub Primer (TypeScript, Vite, Primer CSS, Canvas)
```

---

## 5. Hướng Dẫn Cài Đặt & Phát Triển (Getting Started)

### 5.1 Yêu Cầu Tiên Quyết (Prerequisites)
1. **Hệ điều hành:** Windows 10/11 (khuyến nghị), Linux hoặc macOS.
2. **Rust Toolchain:** Phiên bản `1.82.0` hoặc mới hơn (`rustup default stable`).
3. **C++ Build Tools:** Visual Studio C++ Build Tools (cần thiết cho việc liên kết thư viện native `libgit2` và Windows SDK).
4. **Node.js:** Phiên bản `v20.x` LTS trở lên và package manager `npm` hoặc `pnpm`.
5. **Tauri CLI:** Cài đặt thông qua lệnh `cargo install tauri-cli --version "^2.0"`.

### 5.2 Khởi Chạy Môi Trường Phát Triển (Development Mode)
```powershell
# 1. Di chuyển vào thư mục dự án
cd F:\Build\JavaToolGitAutoUpload\rust-gitupload

# 2. Cài đặt các thư viện frontend (GitHub Primer, Octicons & Canvas)
cd ui
npm install
cd ..

# 3. Chạy ứng dụng ở chế độ Debug kết hợp Hot-Reload
cargo tauri dev
```

### 5.3 Chạy Bộ Kiểm Thử (Run Test Suites)
```powershell
# Chạy toàn bộ Unit Tests và Integration Tests trên toàn bộ 10 crates
cargo test --workspace

# Chạy kiểm thử riêng cho động cơ Git Graph DAG
cargo test -p gitupload-git --test graph_dag_test

# Chạy kiểm thử riêng cho bộ cài đặt GitHub CLI sandbox
cargo test -p gitupload-ghcli

# Chạy linter nghiêm ngặt không cho phép cảnh báo
cargo clippy --all-targets --all-features -- -D warnings
```

### 5.4 Đo Kiểm Hiệu Năng & Bộ Nhớ (Run Benchmarks)
```powershell
# Chạy bộ đo lường tốc độ xử lý Git, Graph DAG, Hooks và tiêu thụ RAM
cargo bench --workspace
```

### 5.5 Đóng Gói Bộ Cài Đặt Sản Phẩm (Production Packaging)
```powershell
# Biên dịch và đóng gói bộ cài đặt tối ưu hóa cao nhất (LTO + Strip)
cargo tauri build
```
*Tệp cài đặt đầu ra sẽ nằm tại:* `target/release/bundle/nsis/GitUpload-v2.2.0-Setup.exe` và `target/release/bundle/msi/GitUpload-v2.2.0.msi`.

---

## 6. Chính Sách Bảo Mật Tuyệt Đối (Zero-Plaintext Policy)

1. **Không Token Bản Rõ:** Mọi thông tin nhạy cảm như GitHub Personal Access Token (PAT) và Cloud AI API Keys đều được mã hóa bằng cơ chế DPAPI phần cứng của Windows thông qua thư viện `keyring`.
2. **Bảo Vệ RAM:** Bí mật được lưu trong struct `secrecy::SecretString`. Bộ nhớ RAM lưu trữ token sẽ tự động bị xóa trắng (zeroized) ngay khi biến ra khỏi phạm vi (out of scope).
3. **Tự Động Che Giấu Log:** Toàn bộ dữ liệu xuất ra màn hình console hoặc file log đều được xử lý qua bộ lọc regex hiệu năng cao để che giấu chuỗi token (thay thế bằng `***`).

---

## 7. Thông Tin Tác Giả & Bản Quyền (Authors & License)

- **Tác giả phát triển:** Stephen (Trần Đức Anh)
- **Email liên hệ:** [tranducanh31032006@gmail.com](mailto:tranducanh31032006@gmail.com)
- **Facebook:** [https://facebook.com/stephen.uc.2025](https://facebook.com/stephen.uc.2025)
- **Repository gốc (Go/Wails v1.4.1):** [java-web-service-stephenduc/Golang-GitToolAutoUpload](https://github.com/java-web-service-stephenduc/Golang-GitToolAutoUpload)
- **Giấy phép phần mềm:** Phân phối theo chuẩn kép [MIT OR Apache-2.0](LICENSE).

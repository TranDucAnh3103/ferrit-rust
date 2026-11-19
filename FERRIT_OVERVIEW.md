# FERRIT — Nền tảng Git Desktop & Quản trị DevSecOps Hiện đại
**Phiên bản:** v2.6.0 Enterprise Edition  
**Công nghệ:** Rust (Tokio + libgit2 + Tauri v2 + Wasmtime) • React 18 • GitHub Primer UI  
**File thực thi chính:** `FERRIT.exe` (~22 MB, Không phụ thuộc Web Server)

---

## 1. Giới thiệu Tổng quan

**FERRIT** là ứng dụng Git Desktop thế hệ mới dành cho lập trình viên cá nhân, đội ngũ kỹ sư doanh nghiệp và môi trường đào tạo. Ứng dụng được thiết kế từ đầu trên nền tảng **100% Pure Rust**, thay thế hoàn toàn các giải pháp nặng nề dựa trên Electron bằng kiến trúc Tauri v2 siêu nhẹ:
- **Bộ nhớ RAM cực thấp**: Chỉ tốn **~33 MB RAM** khi chạy nhàn rỗi (so với 300 - 800 MB của các Git client thông thường).
- **Khởi động tức thì (Cold Start)**: < 200ms.
- **Độc lập hoàn toàn**: Toàn bộ giao diện người dùng và tài nguyên tĩnh được nhúng trực tiếp vào file nhị phân `FERRIT.exe` (sử dụng `custom-protocol`), không cần chạy `npm run dev` hay mở trình duyệt web bên ngoài.
- **Bảo mật chuẩn Enterprise**: Tuyệt đối không lưu token hay khóa API ra file văn bản thô (Plaintext). Mọi dữ liệu nhạy cảm đều được bảo vệ trong **Windows Credential Manager (DPAPI)** và tự động làm sạch trong toàn bộ log hệ thống.

---

## 2. Bản đồ Tính năng Chi tiết (Feature Matrix)

```
                            FERRIT DESKTOP PLATFORM
 ┌───────────────────────┬───────────────────────┬───────────────────────┐
 │     Git Workflows     │  GitHub & DevSecOps   │ Extensibility & Team  │
 ├───────────────────────┼───────────────────────┼───────────────────────┤
 │ • Upload & Sync Hub   │ • AI Security Audit   │ • WASM Plugin Engine  │
 │ • Git Graph (DAG)     │ • Pull Requests       │ • Plugin Marketplace  │
 │ • Multi-Remote Mirror │ • GitHub Actions CI   │ • Webhook Resilience  │
 │ • 3-Way Merge Solver  │ • Issues & Releases   │ • Team Analytics      │
 │ • Semantic Search     │ • SSH Agent & Vault   │ • Enterprise Air-Gap  │
 │ • Stale Branch Purge  │ • GH CLI Sandbox      │ • Headless CLI Mode   │
 └───────────────────────┴───────────────────────┴───────────────────────┘
```

### 2.1. Git Workflow Cốt lõi
1. **Upload & Sync Hub**:
   - Quản lý trạng thái làm việc (Working Tree, Staged, Unstaged) thời gian thực.
   - So sánh Diff trực quan từng dòng code với màu sắc nổi bật.
   - Tích hợp **AI Commit Assistant**: Tự động phân tích thay đổi mã nguồn và tạo thông điệp commit chuẩn **Conventional Commits 1.0.0** (`feat`, `fix`, `docs`, `refactor`...).
   - Đẩy (Push), Kéo (Pull), Đồng bộ (Fetch) và Hủy tác vụ tức thì (Cancellation Token) khi cần dừng truyền tải qua mạng.
2. **Git Graph (DAG Revwalk Visualizer)**:
   - Trực quan hóa cây lịch sử commit dạng đồ thị có hướng không chu trình (DAG) bằng thuật toán phân luồng Swimlane song song.
   - Thao tác nhanh với từng commit: **Checkout**, **Cherry-pick**, **Revert**, **Reset** (Soft / Mixed / Hard), và **Tạo nhánh mới** từ điểm bất kỳ.
3. **Multi-Remote Mirroring & Backup**:
   - Quản lý đồng thời nhiều máy chủ Git từ xa (GitHub, GitLab, Gitea, Bitbucket, hoặc Server nội bộ).
   - Kiểm tra trạng thái ping và độ trễ của từng remote theo cơ chế song song.
   - Tính năng **Push to All Remotes**: Đẩy mã nguồn đồng thời lên tất cả các kho lưu trữ chỉ với 1 click.
4. **3-Way Merge Solver (Giải quyết xung đột hợp nhất)**:
   - Tự động nhận diện xung đột khi Merge hoặc Rebase.
   - Giao diện trực quan 3 bảng: **Base** (tổ tiên chung), **Ours** (nhánh hiện tại), và **Theirs** (nhánh được gộp).
   - **AI Auto-Resolve**: Trí tuệ nhân tạo đọc hiểu logic của cả hai bên và đề xuất phương án giải quyết tối ưu nhất.
   - Cho phép chỉnh sửa thủ công và tạo Commit Merge an toàn.
5. **Semantic Search (Tìm kiếm ngữ nghĩa Commit)**:
   - Vector hóa toàn bộ lịch sử commit bằng SQLite Vector Storage.
   - Tìm kiếm commit bằng ngôn ngữ tự nhiên (tiếng Việt hoặc tiếng Anh) mà không cần nhớ chính xác commit hash hay từ khóa cụ thể.
6. **Branch Cleaner (Dọn dẹp nhánh rác)**:
   - Tự động phát hiện các nhánh Stale (đã merge hoặc không hoạt động > 30 ngày).
   - Kiểm tra an toàn trước khi xóa để tránh mất mã nguồn; hỗ trợ dọn dẹp hàng loạt cả trên Local và Remote.

---

### 2.2. Tích hợp GitHub & Bảo mật DevSecOps
1. **DevSecOps & AI Security Audit**:
   - Trình quét tĩnh tích hợp phát hiện tức thì các khóa API bị hardcode (GitHub PAT, OpenAI, Claude, Google Gemini, AWS keys...).
   - Phát hiện các tệp nhạy cảm vô tình được commit (như `.env`, `.pem`, `.key`).
   - Gợi ý cách vá lỗi và áp dụng bản vá tự động (Auto-Patch).
2. **Quản lý Pull Requests (PR)**:
   - Duyệt danh sách PR với bộ lọc trạng thái (Open, Closed, Merged).
   - Tạo PR mới với tính năng **AI Auto-Summary** tự động viết tóm tắt thay đổi và checklist kiểm thử.
   - Hợp nhất PR trực tiếp từ app với 3 chiến lược: Merge Commit, Squash & Merge, hoặc Rebase & Merge.
3. **Giám sát GitHub Actions CI/CD**:
   - Theo dõi tiến độ chạy Pipeline theo thời gian thực (Queued, In Progress, Completed, Failed).
   - Xem chi tiết log từng bước chạy.
   - Chạy lại các tác vụ lỗi (Rerun Failed Jobs) hoặc hủy workflow run đang treo.
4. **Quản trị Issues & Releases**:
   - Quản lý các vấn đề phát sinh (Issue tracking), tạo và đóng issue nhanh chóng.
   - Đóng gói bản phát hành (Create Release) kèm Git Tag và Release Notes tự động.
5. **SSH Key Manager & Windows SSH Agent**:
   - Quản lý cặp khóa SSH cá nhân (hỗ trợ thuật toán hiện đại **Ed25519** và **RSA 4096-bit**).
   - Tải khóa công khai (Public Key) trực tiếp lên tài khoản GitHub chỉ với một cú nhấp.
   - Tích hợp trình quản lý **SSH Agent** chạy trên Windows Named Pipe, loại bỏ yêu cầu phải gõ passphrase lặp đi lặp lại.
6. **GitHub CLI Sandbox Runner**:
   - Tự động phát hiện hoặc tải phiên bản Portable của GitHub CLI (`gh`).
   - Chạy trong môi trường Sandbox cô lập, tự động đồng bộ tài khoản giữa FERRIT và GitHub CLI.

---

### 2.3. Khả năng Mở rộng (Plugins) & Doanh nghiệp (Enterprise)
1. **WASM Plugin Engine & Sandbox**:
   - Nền tảng plugin chạy trên **WebAssembly (Wasmtime)** với quyền truy cập được giới hạn chặt chẽ (Capabilities: Đọc Git, Ghi Git, Gọi mạng...).
   - Ngăn chặn triệt để mã độc gây hại cho máy tính hoặc rò rỉ mã nguồn.
2. **Community Marketplace**:
   - Chợ plugin tích hợp sẵn cho phép cài đặt các tiện ích mở rộng: Git LFS Guard, Discord/Slack Notifier, Jira Issue Linker...
3. **Hệ thống Webhook Đàn hồi (Webhook Resilience)**:
   - Tự động gửi thông báo HTTP Webhook (POST) tới các hệ thống ngoài khi phát sinh sự kiện Git (Push, Commit, Merge).
   - Ký số bảo mật toàn vẹn bằng **HMAC-SHA256**.
   - Cơ chế tự động thử lại (Exponential Backoff Retry) khi server nhận gặp sự cố tạm thời.
4. **Phân tích Hiệu suất Nhóm & Lớp học (Analytics & Classroom)**:
   - **Contribution Heatmap Calendar**: Biểu đồ nhiệt thể hiện tần suất commit trong 365 ngày.
   - **Velocity & Churn Metrics**: Thống kê số dòng thêm/bớt và tốc độ hoàn thành mã nguồn.
   - **Student Homework Compliance**: Quản lý hạn nộp bài tập theo cú pháp nhánh và bảng xếp hạng tiến độ (Leaderboard).
5. **Chính sách Doanh nghiệp & Môi trường Cách ly (Enterprise Air-Gap)**:
   - Hỗ trợ chế độ **Air-Gapped Mode**: Ngắt hoàn toàn các kết nối mạng bên ngoài, phục vụ các cơ quan, ngân hàng hoặc khu vực an ninh cao.
   - Hỗ trợ thiết lập Corporate Proxy và Whitelist danh sách các miền Git nội bộ được phép tương tác.
6. **Headless CLI Utility (`ferrit-cli`)**:
   - Đi kèm công cụ dòng lệnh độc lập hỗ trợ chạy pipeline CI/CD, xuất báo cáo trạng thái dưới dạng JSON và kiểm tra tuân thủ commit.

---

## 3. Trải nghiệm Người dùng & Giao diện (UI/UX)

- **Ngôn ngữ thiết kế**: GitHub Primer Design System — mang lại cảm giác thân thuộc, gọn gàng và chuẩn mực như giao diện web của GitHub.
- **Biểu tượng Vector chuyên nghiệp**: Sử dụng 100% icon từ thư viện `@primer/octicons-react` và `lucide-react`, loại bỏ hoàn toàn emoji thô.
- **Hệ thống 6 Theme màu sắc đa dạng**:
  1. **Dark Default**: Giao diện tối tiêu chuẩn GitHub.
  2. **Dark Dimmed**: Tông màu xám than dịu mắt cho ban đêm.
  3. **High Contrast**: Tương phản cao hỗ trợ khả năng tiếp cận (Accessibility).
  4. **Light Default**: Giao diện sáng thanh lịch.
  5. **Dracula**: Tông màu tím huyền bí phong cách Dracula kinh điển.
  6. **Monokai**: Tông màu lập trình viên đậm chất hacker.

---

## 4. Cấu trúc Dự án & 10 Crates Chuyên biệt

| Crate | Vai trò & Trách nhiệm | Công nghệ chủ đạo |
| :--- | :--- | :--- |
| **`gitupload-core`** | Khai báo miền dữ liệu chung, cây mã lỗi (`thiserror`), token hủy lệnh. | `serde`, `chrono`, `tokio-util` |
| **`gitupload-security`**| Quản lý khóa trong Windows Vault (DPAPI), SSH Agent, xóa sạch bộ nhớ. | `keyring`, `secrecy`, `zeroize` |
| **`gitupload-storage`** | Cơ sở dữ liệu SQLite nhúng với chế độ WAL, lưu trữ vector và migration. | `rusqlite`, `r2d2` |
| **`gitupload-git`** | Dual-engine Git (in-process libgit2 + CLI), đồ thị DAG, 3-way merge. | `git2`, `tokio::process` |
| **`gitupload-hooks`** | Trình chạy hook native (commit-msg, pre-commit, pre-push). | `regex`, `glob`, `toml` |
| **`gitupload-ghcli`** | Trình phát hiện, tải và chạy GitHub CLI trong sandbox. | `sha2`, `zip`, `semver` |
| **`gitupload-github`** | Giao tiếp GitHub REST/GraphQL (PR, Actions, Issues, Orgs, Releases). | `reqwest`, `eventsource` |
| **`gitupload-ai`** | Bộ điều phối Cloud AI (Gemini 2.0, OpenAI GPT-4o, Claude 3.5, Groq). | `reqwest`, `eventsource-stream` |
| **`gitupload-watcher`**| Giám sát hệ thống tệp và hàng đợi tự động đồng bộ có chống rung. | `notify`, `tokio::sync` |
| **`gitupload-plugins`**| Sandbox chạy extension WebAssembly và quản lý Marketplace. | `wasmtime`, `ring`, `tar` |
| **`gitupload-cli`** | Công cụ dòng lệnh CLI độc lập cho tự động hóa và CI/CD. | `clap`, `colored`, `indicatif` |
| **`gitupload-app`** | Vỏ ứng dụng desktop Tauri v2, xử lý lệnh IPC và kết nối giao diện. | `tauri v2`, `windows-sys` |

---

## 5. Hướng dẫn Khởi chạy & Vị trí Dữ liệu

### 5.1. Chạy Ứng dụng
Bạn chỉ cần mở trực tiếp file thực thi:
- **Đường dẫn**: `F:\Build\JavaToolGitAutoUpload\rust-gitupload\FERRIT.exe`
- Bấm đúp chuột vào file `FERRIT.exe`. Cửa sổ ứng dụng sẽ xuất hiện ngay lập tức với kích thước chuẩn 1280x800 cùng biểu tượng FERRIT trên thanh tác vụ.

### 5.2. Vị trí Lưu trữ Dữ liệu trên Windows
- **Cơ sở dữ liệu SQLite**: `%APPDATA%\ferrit\FERRIT\data\ferrit.db`
- **File nhật ký hệ thống (Logs)**: `%LOCALAPPDATA%\ferrit\FerritDesktop\data\logs\ferrit.log.*`
- **Mật mã & Token**: Lưu trữ trong `Windows Credential Manager` dưới tên dịch vụ `FerritDesktop`.

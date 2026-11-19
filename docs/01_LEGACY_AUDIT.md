# Báo Cáo Kiểm Toán & Đánh Giá Toàn Diện Mã Nguồn Cũ (Legacy Codebase Audit)
## Dự Án: `g-gitupload` (Go/Wails v1.4.1)

> **Bộ Tài Liệu Kỹ Thuật Liên Quan:**
> - [02_ARCHITECTURE.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/02_ARCHITECTURE.md) - Kiến Trúc Hệ Thống Mục Tiêu (Rust / Tauri v2)
> - [03_SRS.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/03_SRS.md) - Đặc Tả Yêu Cầu Phần Mềm (IEEE 830 / ISO 29148)
> - [04_MIGRATION_ROADMAP.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/04_MIGRATION_ROADMAP.md) - Lộ Trình Triển Khai Kỹ Thuật (5 Giai Đoạn)

---

## Mục Lục
1. [Tổng Quan Dự Án & Bản Đồ Mã Nguồn (Executive Summary & Codebase Anatomy)](#1-tổng-quan-dự-án--bản-đồ-mã-nguồn)
2. [Chi Tiết Các Quy Trình Nghiệp Vụ Cốt Lõi (Core Business Workflows)](#2-chi-tiết-các-quy-trình-nghiệp-vụ-cốt-lõi)
   - [2.1 Quy trình đẩy bài tập dạng Thư mục (Folder Push Workflow)](#21-quy-trình-đẩy-bài-tập-dạng-thư-mục-folder-push-workflow)
   - [2.2 Quy trình đẩy bài tập dạng File lẻ Markdown (Single File Push Workflow)](#22-quy-trình-đẩy-bài-tập-dạng-file-lẻ-markdown-single-file-push-workflow)
   - [2.3 Quy trình sao chép Repository & Khởi động IDE (Clone & IDE Launcher Workflow)](#23-quy-trình-sao-chép-repository--khởi-động-ide-clone--ide-launcher-workflow)
   - [2.4 Quy trình tìm kiếm, duyệt cây thư mục & chỉnh sửa file trực tiếp (Search & In-App Editor)](#24-quy-trình-tìm-kiếm-duyệt-cây-thư-mục--chỉnh-sửa-file-trực-tiếp-search--in-app-editor)
   - [2.5 Quy trình kiểm tra & chẩn đoán Token GitHub (Diagnostics & Token Audit)](#25-quy-trình-kiểm-tra--chẩn-đoán-token-github-diagnostics--token-audit)
   - [2.6 Trợ lý ảo Stephen AI & Bộ nhận diện lệnh giọng nói (AI Assistant & Voice Dispatch)](#26-trợ-lý-ảo-stephen-ai--bộ-nhận-diện-lệnh-giọng-nói-ai-assistant--voice-dispatch)
3. [Kiến Trúc Backend Golang & Phân Tích Chi Tiết Từng Module](#3-kiến-trúc-backend-golang--phân-tích-chi-tiết-từng-module)
4. [Kiến Trúc Frontend & Phân Tích Giao Diện UI/UX](#4-kiến-trúc-frontend--phân-tích-giao-diện-uiux)
5. [Mô Hình Dữ Liệu & Danh Mục Trạng Thái Lưu Trữ (Data Models & Schemas)](#5-mô-hình-dữ-liệu--danh-mục-trạng-thái-lưu-trữ-data-models--schemas)
6. [Hạn Chế Kỹ Thuật, Nút Thắt Hiệu Năng & Nợ Kỹ Thuật (Technical Debt & Bottlenecks)](#6-hạn-chế-kỹ-thuật-nút-thắt-hiệu-năng--nợ-kỹ-thuật-technical-debt--bottlenecks)
7. [Đánh Giá Toàn Diện Lỗ Hổng Bảo Mật (Security Vulnerability Assessment)](#7-đánh-giá-toàn-diện-lỗ-hổng-bảo-mật-security-vulnerability-assessment)
8. [Đối Chiếu Đặc Tả SRS v2.0 & Định Hướng Viết Lại Bằng Rust (Gap Analysis & Rust Roadmap)](#8-đối-chiếu-đặc-tả-srs-v20--định-hướng-viết-lại-bằng-rust-gap-analysis--rust-roadmap)

---

## 1. Tổng Quan Dự Án & Bản Đồ Mã Nguồn

### 1.1 Mục Đích & Bối Cảnh
`g-gitupload` (phiên bản hiện tại: `v1.4.1`) là tiện ích desktop dành cho hệ điều hành Windows được phát triển nhằm tự động hóa quy trình nộp bài tập của học viên lập trình lên nền tảng GitHub. Công cụ giải quyết các bài toán thủ công lặp đi lặp lại:
- Khởi tạo Git repository cục bộ và cấu hình danh tính (`user.name`, `user.email`).
- Gọi GitHub REST API để tạo repository trên tài khoản cá nhân hoặc tổ chức (Organization).
- Đẩy mã nguồn thông qua lệnh `git push` cưỡng bức (`--force`).
- Quản lý kho bài tập (tìm kiếm, xem trước nội dung README, chỉnh sửa tệp trực tiếp).
- Hỗ trợ học viên clone bài tập về máy và mở nhanh qua các IDE phổ biến (VS Code, IntelliJ IDEA, Cursor, Antigravity IDE).
- Tích hợp chatbot hỗ trợ người học (Stephen AI) kết nối mô hình ngôn ngữ lớn (LLM) qua OpenRouter / Xkiro API.

### 1.2 Công Nghệ Nền Tảng (Tech Stack)
- **Framework nền tảng:** [Wails v2](https://wails.io/) (`v2.12.0`) kết hợp WebView2 trên Windows.
- **Backend:** Go 1.23.0 (Windows `amd64`), phụ thuộc vào `kernel32.dll`, `user32.dll`, và CLI `git.exe` cài đặt trên máy chủ hệ điều hành.
- **Frontend:** Vanilla JavaScript (ES Modules), HTML5, CSS3, đóng gói bằng Vite 3.0.7, bộ icon vector Lucide.
- **Cơ chế lưu trữ:** Tập tin JSON phẳng (`config.json`, `history.json`), tập tin `.env`, và trình duyệt `localStorage`.

### 1.3 Bản Đồ Thư Mục & Vai Trò Các Tệp Tin

```
f:\Build\JavaToolGitAutoUpload\g-gitupload
├── .env                       # Cấu hình dự phòng chứa token GitHub, API key (NGUY HIỂM: Lưu plaintext)
├── .gitignore                  # Cấu hình bỏ qua tệp của Git
├── ai_service.go              # Xử lý chat AI, chẩn đoán lỗi, phân tích intent giọng nói, quét thư mục
├── app.go                     # Khởi tạo App struct và nhúng HUONG_DAN.txt
├── clone_service.go           # Logic clone repository, phát hiện IDE và khởi động ứng dụng bên ngoài
├── config/
│   └── config.go              # Nạp/lưu cấu hình hệ thống từ %APPDATA% hoặc .env
├── docs/
│   └── SRS_v2.md              # Tài liệu phác thảo yêu cầu kỹ thuật v2.0
├── git/
│   └── runner.go              # Trình bao bọc (wrapper) chạy các lệnh CLI git.exe, bắt log & tiến độ
├── github/
│   └── client.go              # Client HTTP gọi GitHub REST API (v3) với kết nối dùng chung
├── go.mod / go.sum            # Khai báo dependency Go
├── history/
│   └── tracker.go             # Lưu trữ và đọc thông tin repository vừa đẩy gần nhất
├── HUONG_DAN.txt              # Văn bản hướng dẫn người dùng bằng tiếng Việt
├── main.go                    # Entry point: Single Instance Mutex Win32, Win32 MessageBox, chạy Wails
├── push_service.go            # Điều phối tiến trình push bài tập (thư mục & file lẻ)
├── README.md                  # Tài liệu giới thiệu dự án v1.4.1
├── RELEASE_GUIDE.md           # Hướng dẫn tạo release GitHub
├── search_service.go          # Tìm kiếm repository, duyệt file explorer, sửa file, thống kê ngôn ngữ
├── system_service.go          # Mở trình duyệt, hộp thoại chọn file/folder, render markdown, profile
├── wails.json                 # Cấu hình dự án Wails v2
└── frontend/
    ├── index.html             # Khung giao diện đơn trang (SPA) gồm 6 tab, modal và chat panel
    ├── package.json           # Khai báo dependency Vite
    └── src/
        ├── app.css / style.css# Giao diện Glassmorphism, Dark/Light theme, bố cục lưới
        ├── main.js            # Điểm khởi chạy frontend, gắn kết sự kiện toàn cục và chuyển tab
        ├── components/        # Console.js, Modal.js, ProfileCard.js, ProgressBar.js, RepoCard.js, Toast.js
        ├── services/          # PushService.js, CloneService.js, ConfigService.js, GitHubService.js,
        │                      # HistoryService.js, SearchService.js, ChatService.js, chatRules.js, VoiceService.js
        └── views/
            └── GuideTab.js    # Phân tích văn bản HUONG_DAN.txt thành tab tài liệu ngang
```

---

## 2. Chi Tiết Các Quy Trình Nghiệp Vụ Cốt Lõi

### 2.1 Quy trình đẩy bài tập dạng Thư mục (Folder Push Workflow)

```
[Người dùng chọn thư mục / Kéo thả]
                 │
                 ▼
     [Kiểm tra tính hợp lệ PATH]
                 │
                 ▼
[Tải Cấu hình: Token, Username, Naming Pattern]
                 │
                 ▼
[Xác định Tên Repository & Commit Message]
 ├── Chế độ SESSION_EX: pattern %s (vd: session15-ex01-homework)
 ├── Chế độ MINI_PROJECT: session{N}-miniproject-{slug}
 └── Chế độ CUSTOM_REPOSITORY: Tên tự do đã chuẩn hóa
                 │
                 ▼
[Gọi GitHub REST API: POST /user/repos hoặc /orgs/{org}/repos]
 ├── Nếu mã 201: Tạo mới thành công
 ├── Nếu mã 422 ("already exists"):
 │     └── GET repo: Nếu r.Size == 0 (repo rỗng) -> Tái sử dụng
 │                   Nếu r.Size > 0 -> BÁO LỖI: Repo đã có dữ liệu
                 │
                 ▼
[Thực thi chuỗi lệnh Git CLI trong thư mục người dùng]
 ├── 1. git init
 ├── 2. git config gc.auto 0
 ├── 3. git config user.name {Name}
 ├── 4. git config user.email {Email}
 ├── 5. Đảm bảo .env nằm trong .gitignore (tạo hoặc append)
 ├── 6. git add .
 ├── 7. git commit --allow-empty -m {commitMsg}
 ├── 8. git branch -M main
 ├── 9. git remote remove origin (bỏ qua lỗi nếu chưa có)
 ├── 10. git remote add origin https://{token}@github.com/{owner}/{repo}.git
 └── 11. git push -u origin main --force --progress
                 │
                 ▼
[Phân tích stdout/stderr bắt % tiến độ qua regex `Writing objects: (\d+)%`]
                 │
                 ▼
[Lưu Lịch sử vào %APPDATA%\GGitUpload\history.json (Chỉ lưu 1 repo duy nhất)]
                 │
                 ▼
[Kích hoạt Garbage Collector goruntime.GC() và phát Event thông báo UI]
```

### 2.2 Quy trình đẩy bài tập dạng File lẻ Markdown (Single File Push Workflow)
Trong phiên bản `v1.4.1`, ứng dụng giải quyết tình huống học sinh chỉ nộp 1 file bài tập (thường là file `.md`) mà không muốn làm rác máy tính bằng việc tạo thư mục `.git` trong thư mục cá nhân (như Desktop, Downloads):
1. **Phát hiện tệp tin:** Khi người dùng chọn đường dẫn không phải thư mục (`!info.IsDir()`).
2. **Khởi tạo môi trường tạm thời:**
   - Tạo thư mục tạm hệ điều hành: `os.MkdirTemp("", "g-gitupload-file-*")`.
   - Đăng ký hàm hủy thư mục tạm: `defer os.RemoveAll(tempDir)`.
   - Sao chép tệp nguồn vào thư mục tạm bằng `io.Copy`.
3. **Phân tích cú pháp tên tệp (Auto-Parsing):**
   - Áp dụng biểu thức chính quy: `(?i)SS(\d+)[_-](?:HW|EX)(\d+)` trên tên file (ví dụ: `SS01_HW02_NguyenVanA_PTIT123.md`).
   - Tự động trích xuất: `session = 01`, `exercise = 02`.
   - Tách chuỗi theo dấu gạch dưới `_` để bóc tách: `{name}` và `{student_id}`.
4. **Tạo Commit Message động:**
   - Thay thế các thẻ giữ chỗ: `{filename}`, `{filename_no_ext}`, `{session}`, `{exercise}`, `{name}`, `{student_id}` vào mẫu cấu hình `FileCommitMsgPattern`.
5. **Tùy chọn đồng bộ tên Repo với Commit Message:**
   - Nếu bật cờ `SyncFileRepoNameWithCommit`: Tạo slug từ commit message (bỏ ký tự lạ, thay khoảng trắng bằng gạch nối) để làm tên repository GitHub.
6. **Thực thi Git Push:** Toàn bộ quá trình `git init` và `git push` diễn ra bên trong thư mục tạm, bảo toàn tính nguyên vẹn của thư mục gốc của học viên.

### 2.3 Quy trình sao chép Repository & Khởi động IDE (Clone & IDE Launcher Workflow)
1. **Lấy danh sách hoặc nhập URL:**
   - Nguồn GitHub: Gọi `GetMyRepos()` tải 100 repo mới nhất của tài khoản/tổ chức vào thẻ select.
   - Nguồn Custom: Người dùng dán liên kết HTTPS tùy ý.
2. **Tiêm Token Xác Thực:**
   - Nếu là URL GitHub HTTPS và trong cấu hình có `GithubToken`, URL được biến đổi thành:
     `https://{token}@github.com/{owner}/{repo}.git`
3. **Chạy Subprocess Clone:**
   - Chạy lệnh: `git clone --progress <targetURL> <destFolder>`.
   - Chạy ngầm không hiển thị cửa sổ CMD (`HideWindow: true`, `GIT_TERMINAL_PROMPT=0`).
   - Quét luồng stdout/stderr gộp thông qua hàm phân tách `SplitByNewlineOrCR`.
   - Phân tích tiến độ thời gian thực qua regex: `(Receiving objects|Resolving deltas):\s+(\d+)%`.
   - Che giấu token nhạy cảm trong luồng log gửi lên giao diện người dùng bằng chuỗi `******`.
4. **Tự Động Quét Môi Trường Lập Trình (IDE Auto-Detection):**
   Ứng dụng quét trực tiếp hệ thống tệp Windows để kiểm tra sự tồn tại của các trình soạn thảo mã nguồn:
   - **VS Code:** Kiểm tra `%LOCALAPPDATA%\Programs\Microsoft VS Code\Code.exe`, `%ProgramFiles%\Microsoft VS Code\Code.exe`, hoặc lệnh `code --version`.
   - **IntelliJ IDEA:**
     - Bản cài độc lập: Quét thư mục `%ProgramFiles%\JetBrains\*\bin\idea64.exe`.
     - Bản JetBrains Toolbox: Quét `%LOCALAPPDATA%\JetBrains\Toolbox\apps\IDEA-U\*\bin\idea64.exe` và `IDEA-C`.
   - **Cursor:** Kiểm tra `%LOCALAPPDATA%\Programs\cursor\Cursor.exe` hoặc lệnh `cursor --version`.
   - **Antigravity IDE:** Kiểm tra `%USERPROFILE%\.gemini\antigravity-ide\Antigravity.exe` hoặc `%LOCALAPPDATA%\Programs\antigravity-ide\Antigravity.exe`.
5. **Khởi Chạy Dự Án:**
   Khi bấm nút tương ứng, ứng dụng gọi lệnh thực thi chạy ngầm với cờ `CreationFlags: 0x08000000` (CREATE_NO_WINDOW) để mở IDE trỏ trực tiếp vào thư mục mã nguồn vừa clone.

### 2.4 Quy trình tìm kiếm, duyệt cây thư mục & chỉnh sửa file trực tiếp (Search & In-App Editor)
1. **Tìm Kiếm Repository:**
   - Nếu có từ khóa tìm kiếm: Gọi GitHub Search API `GET /search/repositories?q={query}+{user|org}:{owner}`.
   - Nếu không có từ khóa: Gọi `GET /user/repos` hoặc `GET /orgs/{owner}/repos` phân trang (12 repo/trang).
   - Sắp xếp kết quả phía client (`SortRepositories`):
     - `empty`: Các repo chưa có code (`default_branch == ""`) được đẩy lên đầu.
     - `new`: Các repo có `pushed_at` trong vòng 24 giờ được gắn nhãn `[NEW]` và ưu tiên hiển thị.
2. **Duyệt Thư Mục Từ Xa (Remote File Explorer):**
   - Gọi API `GET /repos/{owner}/{repo}/contents/{path}`.
   - Hiển thị danh sách file và folder, tính dung lượng file, hỗ trợ điều hướng thư mục cha (`..`).
3. **Xem README.md Render Sẵn:**
   - Gọi `GET /repos/{owner}/{repo}/readme` với header `Accept: application/vnd.github.html` để nhận mã HTML đã render từ GitHub và chèn trực tiếp vào DOM.
4. **Trình Soạn Thảo File Trực Tiếp (In-App Editor):**
   - Gọi API lấy nội dung file dạng Base64 và mã `SHA` của commit hiện tại.
   - Giải mã Base64 sang UTF-8 hiển thị trên textarea.
   - Khi lưu: Mã hóa nội dung sang Base64 và gửi request `PUT /repos/{owner}/{repo}/contents/{path}` kèm `sha` và commit message để commit trực tiếp lên nhánh mặc định.

### 2.5 Quy trình kiểm tra & chẩn đoán Token GitHub (Diagnostics & Token Audit)
- Gọi API `GET /user` với token người dùng.
- Trích xuất thông tin header HTTP:
  - `X-OAuth-Scopes`: Kiểm tra quyền `repo`, `delete_repo`, `admin:org` / `write:org`.
  - `X-RateLimit-Limit`, `X-RateLimit-Remaining`, `X-RateLimit-Reset`: Tính toán thời gian khôi phục hạn mức gọi API.
- Bóc tách payload JSON: Thống kê tổng số public repos, private repos, dung lượng ổ đĩa GitHub (`disk_usage`), trạng thái xác thực 2 bước (`two_factor_authentication`), và gói dịch vụ (`plan_name`).

### 2.6 Trợ lý ảo Stephen AI & Bộ nhận diện lệnh giọng nói (AI Assistant & Voice Dispatch)
1. **Bộ lọc Intent Cục Bộ (chatRules.js & Go Rules):**
   - Lọc chào hỏi tĩnh (`GREETINGS`) -> Phản hồi lập tức không tốn token LLM.
   - Lọc câu hỏi ngoài lề (`OFF_TOPIC_KEYWORDS` như bóng đá, thời tiết, chính trị) -> Từ chối lịch sự.
   - Câu hỏi tác giả/nhà phát triển -> Trả về liên kết GitHub và thông tin của tác giả Stephen.
   - Lệnh nhanh người dùng (`clone <url>`, `push <folder>`) -> Phân giải hành vi tự động.
2. **Gọi LLM Qua Cổng OpenRouter / Xkiro:**
   - Payload gồm `SYSTEM_PROMPT`, tài liệu `HUONG_DAN.txt` nhúng sẵn, tóm tắt trạng thái hệ thống hiện tại (`AppContext`), và 30 dòng log console gần nhất.
   - Hỗ trợ mô hình mặc định: `openai/gpt-4o-mini:free` hoặc mô hình cấu hình như `minimax/minimax-m2.7-highspeed`.
   - Nếu API key có tiền tố `sk-xt-`: Tự động định tuyến sang máy chủ `https://api.xkiro.com/v1/chat/completions`.
   - Phân tích cú pháp thẻ hành động của Agent: `[AGENT_ACTION:clone:url=...]` hoặc `[AGENT_ACTION:push:folder=...]` để tự động chuyển tab và kích hoạt tiến trình tương ứng trên giao diện.
3. **Tự Động Bắt Lỗi & Chẩn Đoán Khi Push Thất Bại:**
   - Khi Backend phát sinh sự kiện `push_error`, nút FAB tròn của Stephen AI sẽ nhấp nháy badge cảnh báo màu đỏ (`alert-circle`).
   - Khi người dùng click mở khung chat, Stephen AI tự động phân tích dòng log lỗi vừa xảy ra và đưa ra lời khuyên khắc phục cho người học.
4. **Nhận Diện Giọng Nói (Voice Command):**
   - Tích hợp Web Speech API (`SpeechSynthesisUtterance`) phát âm thanh phản hồi bằng tiếng Việt.
   - Phân tích biểu thức chính quy nhận diện giọng nói tiếng Việt cho lệnh điều hướng tab và lệnh đẩy bài (`push buổi (\d+) bài (\d+)`).
   - Dự phòng (Fallback): Gửi câu transcript sang LLM để phân tích cú pháp JSON theo schema: `{action, params, response}`.

---

## 3. Kiến Trúc Backend Golang & Phân Tích Chi Tiết Từng Module

### 3.1 `main.go` & `app.go`
- **Khóa Ứng Dụng Đơn Phiên (Single Instance Mutex):**
  Sử dụng Win32 API thông qua `syscall.NewLazyDLL("kernel32.dll").NewProc("CreateMutexW")` với tên mutex toàn cục `Local\GGitUploadSingleInstanceMutex`. Nếu mã lỗi trả về là `ERROR_ALREADY_EXISTS` (183), ứng dụng gọi Win32 `MessageBoxW` hiển thị thông báo chặn chạy trùng lặp và thoát ngay lập tức trước khi tải WebView2.
- **Khởi Tạo Wails:**
  Cấu hình kích thước cửa sổ 1100x720, màu nền tối `#0F172A`, bật chế độ kéo thả tệp native (`EnableFileDrop: true`, `DisableWebViewDrop: true`), liên kết đối tượng `app` duy nhất.

### 3.2 `git/runner.go`
- **Kiểm tra Git khả dụng:** Thực thi lệnh `git --version` ngầm với cờ `CreationFlags: 0x08000000` (CREATE_NO_WINDOW).
- **Hàm Tách Dòng `SplitByNewlineOrCR`:**
  Git CLI khi đẩy hoặc clone với cờ `--progress` xuất ra các ký tự Carriage Return (`\r`) để ghi đè dòng phần trăm trên terminal. Trình phân tách scanner chuẩn của Go mặc định bỏ qua `\r` gây nghẽn buffer. Module này tùy chỉnh thuật toán `bufio.SplitFunc` để tách cả `\r`, `\n` và `\r\n`.
- **Ẩn Token Trong Log Console:**
  Áp dụng biểu thức chính quy `https://[^@\s"]+@github\.com` để thay thế token PAT thành `https://***@github.com` trước khi phát event `git_log` lên UI.
- **Bảo Vệ Tệp `.env` Cục Bộ:**
  Hàm `ensureEnvIgnored(dir)` tự động quét tệp `.gitignore` trong thư mục bài tập của người dùng, nếu chưa có sẽ tạo mới hoặc append thêm dòng `.env` nhằm ngăn chặn học sinh vô tình đẩy token nhạy cảm lên GitHub.

### 3.3 `github/client.go`
- **Tái Sử Dụng Kết Nối HTTP (Connection Pooling):**
  Sử dụng một biến toàn cục `httpClient = &http.Client{Timeout: 15 * time.Second}` dùng chung, duy trì kết nối TCP Keep-Alive để tránh lãng phí socket hệ thống khi thực hiện nhiều lệnh gọi REST API liên tiếp.
- **Cơ Chế Tái Sử Dụng Repository Trống:**
  Khi nhận mã lỗi HTTP 422 (Unprocessable Entity) do repository đã tồn tại, hàm `CreateRepository` sẽ thực hiện một truy vấn `GetRepository`. Nếu thuộc tính `Size == 0` (repo hoàn toàn chưa có commit nào), hệ thống chấp nhận tái sử dụng repo này để push thay vì dừng lại báo lỗi.

### 3.4 `push_service.go`
- **Quản Lý Đồng Thời (Concurrency Mutex):**
  Sử dụng `sync.Mutex` (`pushMu`) và hàm hủy `pushCancel context.CancelFunc`. Nếu người dùng cố tình bấm Push nhiều lần, hệ thống chặn lại với thông báo "một tiến trình push đang chạy".
- **Hủy Tiến Trình An Toàn (Cancellation):**
  Toàn bộ các lệnh gọi Git đều gắn liền với `pushCtx`. Khi hàm `CancelPush` được gọi, `pushCancel()` sẽ phát tín hiệu ngắt toàn bộ tiến trình con của hệ điều hành ngay lập tức.
- **Thu Gom Rác Chủ Động:**
  Trong khối `defer` của `StartPush`, ứng dụng chủ động gọi `goruntime.GC()` để dọn dẹp các đối tượng buffer và trả bộ nhớ Heap về cho Windows.

### 3.5 `clone_service.go`
- Điều phối lệnh `git clone --progress` với môi trường `GIT_TERMINAL_PROMPT=0` để ngăn Git dừng lại chờ nhập mật khẩu trên terminal ẩn.
- Xử lý che dấu token nhạy cảm trong luồng log clone.
- Triển khai thuật toán quét thư mục tìm kiếm bộ thực thi của VS Code, IntelliJ IDEA, Cursor, và Antigravity IDE.

### 3.6 `search_service.go`
- Cung cấp các wrapper tương tác với `github/client.go` cho tính năng tìm kiếm, phân trang động, lấy danh sách ngôn ngữ lập trình của repository, duyệt cây thư mục từ xa, lấy nội dung tệp tin mã hóa Base64 và commit thay đổi ngược lại GitHub.

### 3.7 `system_service.go`
- Mở URL trên trình duyệt web mặc định của Windows bằng lệnh:
  `rundll32 url.dll,FileProtocolHandler <url>`.
- Giao tiếp hộp thoại hệ thống Wails `OpenDirectoryDialog` và `OpenFileDialog`.
- Hỗ trợ tự động điền thư mục gần nhất (`getRecentUploadDir`) bằng cách truy vết từ lịch sử `history.json`.
- Tải tài liệu `HUONG_DAN.txt` từ GitHub raw URL (nếu có internet) hoặc sử dụng nội dung nhúng `guideContent`, sau đó gọi API `https://api.github.com/markdown` để render sang HTML chuẩn GFM.

### 3.8 `ai_service.go`
- Xây dựng client kết nối API OpenRouter / Xkiro với timeout 60 giây.
- Thu thập ngữ cảnh hệ thống (`GetAppContext`) truyền cho AI.
- Phân giải biểu thức chính quy nhận diện giọng nói và quét đệ quy các thư mục lân cận (`FindMatchingDirectory`) dựa trên mẫu tên buổi học và bài tập.

### 3.9 `config/config.go` & `history/tracker.go`
- **Cấu hình phân tầng:**
  1. Ưu tiên đọc `%APPDATA%\GGitUpload\config.json`.
  2. Dự phòng: Đọc tập tin `.env` cùng thư mục thực thi với parser thủ công không phụ thuộc thư viện thứ 3.
- **Lịch sử đẩy bài:**
  Lưu trữ tại `%APPDATA%\GGitUpload\history.json` dưới dạng một bản ghi duy nhất `LastPushed *PushedRepo`.

---

## 4. Kiến Trúc Frontend & Phân Tích Giao Diện UI/UX

### 4.1 Bố Cục Tổng Thể & Giao Diện
- **Thiết kế Glassmorphism & Tối Giản:** Sử dụng biến CSS (`--panel-bg`, `--panel-border`, `--accent-blue`, `--accent-purple`, `--text-primary`), hỗ trợ 3 chế độ chủ đề: Sáng (Light), Tối (Dark), và Đồng bộ Hệ thống (System).
- **Điều Hướng Sidebar:** Menu gồm 6 mục được sắp xếp khoa học theo hành vi người dùng:
  `Đẩy bài tập` -> `Sao chép Repo` -> `Tìm kiếm Repo` -> `Báo cáo GitHub` -> `Cách hoạt động` -> `Cài đặt`.
- **Card Thông Tin GitHub:** Hiển thị avatar, tên, số lượng followers và public repos ở chân sidebar khi có cấu hình hợp lệ.
- **Chặn Điều Hướng Webview Gây Lỗi:**
  Lắng nghe sự kiện `dragover` và `drop` toàn cục trên `window` và gọi `e.preventDefault()` để triệt tiêu lỗi WebView2 điều hướng đến trang `about:blank#blocked`.
- **Bộ Chặn Liên Kết Toàn Cục (Global Link Interceptor):**
  Tất cả các thẻ `<a>` có liên kết `http://` hoặc `https://` đều bị chặn hành vi mặc định và chuyển hướng mở qua trình duyệt mặc định của hệ điều hành thông qua Go binding `OpenBrowser`.

### 4.2 Chi Tiết Các Component Frontend
- **Console Monitor Log (`Console.js`):**
  Tự động cuộn theo dòng dữ liệu mới (`autoScroll = true`). Cho phép người dùng cuộn ngược lại xem log cũ mà không bị giật màn hình nếu khoảng cách cuộn vượt ngưỡng 15px.
- **Thanh Tiến Độ (`ProgressBar.js`):**
  Cập nhật mượt mà phần trăm và nhãn trạng thái từ backend (`Tạo repository...` -> `Đồng bộ git...` -> `Đang dọn dẹp...`).
- **Thẻ Repository (`RepoCard.js`):**
  Hiển thị trạng thái kho lưu trữ thông qua các huy hiệu trực quan: `[EMPTY]` (repo chưa có nhánh mặc định), `[NEW]` (repo được push trong vòng 24 giờ qua), và nhãn ngôn ngữ lập trình chiếm tỷ lệ cao nhất.
- **Bảng Chat Stephen AI (`ChatService.js`):**
  Nút FAB kéo thả tự do trên màn hình (`makeElementDraggable`), hỗ trợ kéo dãn kích thước bảng chat sang ngang lên đến 50% độ rộng màn hình (`setupChatResizer`), nút xóa lịch sử chat, và nút copy nội dung câu hỏi khi hover.

---

## 5. Mô Hình Dữ Liệu & Danh Mục Trạng Thái Lưu Trữ (Data Models & Schemas)

### 5.1 Bảng Cấu Trúc Dữ Liệu Go & JSON

#### `AppConfig` (`config/config.go`)
```go
type AppConfig struct {
    GithubToken                 string `json:"github_token"`
    GithubUsername              string `json:"github_username"`
    GithubOrgName               string `json:"github_org_name"`
    NamingPattern               string `json:"naming_pattern"`
    DeleteKey                   string `json:"delete_key"`
    ThemeMode                   string `json:"theme_mode"`
    GitCommitName               string `json:"git_commit_name"`
    GitCommitEmail              string `json:"git_commit_email"`
    RepoPrivate                 bool   `json:"repo_private"`
    AutoPush                    bool   `json:"auto_push"`
    DefaultCommitMsg            string `json:"default_commit_msg"`
    FileCommitMsgPattern        string `json:"file_commit_msg_pattern"`
    SyncFileRepoNameWithCommit  bool   `json:"sync_file_repo_name_with_commit"`
    AiApiKey                    string `json:"ai_api_key"`
    AiModel                     string `json:"ai_model"`
}
```

#### `PushedRepo` & `History` (`history/tracker.go`)
```go
type PushedRepo struct {
    RepoName  string `json:"repo_name"`
    RepoUrl   string `json:"repo_url"`
    PushedAt  string `json:"pushed_at"`
    LocalPath string `json:"local_path"`
}

type History struct {
    LastPushed *PushedRepo `json:"last_pushed"`
}
```

#### `TokenReport` (`github/client.go`)
```go
type TokenReport struct {
    Scopes          []string `json:"scopes"`
    RateLimit       int      `json:"rate_limit"`
    RateRemaining   int      `json:"rate_remaining"`
    RateReset       string   `json:"rate_reset"`
    PublicRepos     int      `json:"public_repos"`
    PrivateRepos    int      `json:"private_repos"`
    DiskUsage       int      `json:"disk_usage"`
    PlanName        string   `json:"plan_name"`
    TwoFactor       bool     `json:"two_factor_authentication"`
    HasRepoScope    bool     `json:"has_repo_scope"`
    HasDeleteScope  bool     `json:"has_delete_scope"`
    HasOrgScope     bool     `json:"has_org_scope"`
    Username        string   `json:"username"`
    AvatarUrl       string   `json:"avatar_url"`
    Email           string   `json:"email"`
}
```

#### `RepositoryInfo` (`github/client.go`)
```go
type RepositoryInfo struct {
    Name            string `json:"name"`
    HtmlUrl         string `json:"html_url"`
    Size            int    `json:"size"`
    Description     string `json:"description"`
    StargazersCount int    `json:"stargazers_count"`
    ForksCount      int    `json:"forks_count"`
    WatchersCount   int    `json:"watchers_count"`
    Private         bool   `json:"private"`
    CreatedAt       string `json:"created_at"`
    UpdatedAt       string `json:"updated_at"`
    PushedAt        string `json:"pushed_at"`
    DefaultBranch   string `json:"default_branch"`
}
```

#### `AppContext` & `VoiceResult` (`ai_service.go`)
```go
type AppContext struct {
    Config         *config.AppConfig     `json:"config"`
    LastPushedRepo *history.PushedRepo   `json:"last_pushed_repo"`
    GitAvailable   bool                  `json:"git_available"`
    GitHubProfile  *github.GitHubProfile `json:"github_profile,omitempty"`
    Platform       string                `json:"platform"`
}

type VoiceResult struct {
    Action   string            `json:"action"`   // "push", "clone", "search", "navigate", "help", "unknown"
    Params   map[string]string `json:"params"`   // e.g. {"session": "15", "ex": "01"}
    Response string            `json:"response"` // Câu phản hồi tiếng Việt
}
```

### 5.2 Nơi Lưu Trữ Dữ Liệu
1. **`%APPDATA%\GGitUpload\config.json`:** Lưu toàn bộ thông số cấu hình của ứng dụng.
2. **`%APPDATA%\GGitUpload\history.json`:** Lưu repo được push gần đây nhất.
3. **`.env`:** Nạp dự phòng khi `config.json` chưa được khởi tạo.
4. **Browser `localStorage`:** Lưu cờ bật/tắt của trợ lý ảo (`bot_chat_enabled`) và trợ lý giọng nói (`voice_chat_enabled`).
5. **Bộ nhớ RAM runtime:** Các biến trạng thái tiến trình `pushCancel`, `window.__pendingAgentPushFolder`, `chatHistory`.

---

## 6. Hạn Chế Kỹ Thuật, Nút Thắt Hiệu Năng & Nợ Kỹ Thuật (Technical Debt & Bottlenecks)

### 6.1 Giới Hạn Lưu Trữ Lịch Sử (Single-Item History Loss)
- **Vấn đề:** Struct `History` chỉ có duy nhất trường `LastPushed *PushedRepo`. Mỗi khi người dùng đẩy bài tập mới, toàn bộ dữ liệu của lần đẩy bài trước đó bị ghi đè hoàn toàn.
- **Hệ quả:** Ứng dụng không thể xây dựng biểu đồ đóng góp (Contribution Heatmap), không có nhật ký kiểm toán (Audit Trail) để tra cứu lại các bài đã nộp trong quá khứ, và vi phạm nghiêm trọng yêu cầu của Epic 4 trong tài liệu SRS v2.0.

### 6.2 Kiến Trúc Đơn Tài Khoản (Single-Profile Lock-in)
- **Vấn đề:** `AppConfig` chỉ lưu trữ 1 cặp `github_token` và `github_username` duy nhất.
- **Hệ quả:** Học viên có nhiều tài khoản (tài khoản cá nhân để làm bài tập, tài khoản GitHub tổ chức trường học, tài khoản GitLab công ty) buộc phải xóa đi nhập lại token và username mỗi khi chuyển đổi ngữ cảnh.

### 6.3 Phụ Thuộc Vào Tiến Trình Ngoài Của Hệ Điều Hành (`git.exe` Process Spawning)
- **Vấn đề:** Ứng dụng không sử dụng thư viện Git nhúng (như `go-git` hay `libgit2`) mà phụ thuộc 100% vào việc khởi tạo subprocess `os/exec.CommandContext("git", ...)`. Mỗi tiến trình push phải gọi tuần tự từ 7 đến 11 subprocess con riêng biệt (`init`, `config`, `add`, `commit`, `branch`, `remote`, `push`).
- **Nút thắt hiệu năng:** Trên Windows, chi phí khởi tạo process (`CreateProcessW`) rất tốn kém (khoảng 30-100ms cho mỗi process con). Nếu máy tính của người dùng có trình diệt virus (Windows Defender / Antivirus real-time scan), việc tạo và hủy liên tục các process `git.exe` gây trễ đáng kể và có thể bị khóa file ngẫu nhiên.
- **Điểm gãy (Single Point of Failure):** Nếu người dùng chưa cài đặt Git hoặc Git chưa được thêm vào biến môi trường hệ thống `PATH`, ứng dụng sẽ tê liệt hoàn toàn các chức năng cốt lõi.

### 6.4 Đẩy Đè Lịch Sử Nguy Hiểm (`git push --force`)
- **Vấn đề:** Câu lệnh cố định trong `git/runner.go` dòng 184 là:
  ```go
  runSingleCommand(ctx, folderPath, onLog, onProgress, "git", "push", "-u", "origin", "main", "--force", "--progress")
  ```
- **Hệ quả:** Việc dùng cờ cưỡng bức `--force` mà không có cơ chế kiểm tra trạng thái remote có thể xóa sạch các commit hoặc branch khác mà giảng viên hoặc đồng đội đã tạo trên repository từ trước mà không thể cứu vãn dữ liệu.

### 6.5 Thuật Toán Tìm Kiếm Thư Mục Tự Động Mang Tính Suy Diễn Dễ Sai Lệch (`FindMatchingDirectory`)
- **Vấn đề:** Hàm `FindMatchingDirectory` trong `ai_service.go` quét đệ quy thư mục Desktop, Documents và thư mục cha của lần push trước để tìm các tên thư mục khớp với chuỗi con: `"session" + num`, `"buoi" + num`, `"bài " + num`.
- **Hệ quả:** Thao tác quét đĩa này chạy trên luồng chính, gây nghẽn I/O nếu thư mục Desktop có dung lượng lớn hoặc chứa hàng chục nghìn file. Hơn nữa, việc so khớp chuỗi lỏng lẻo dễ chọn nhầm thư mục không mong muốn của người dùng.

### 6.6 Trạng Thái Giao Diện Frontend Rời Rạc & Thao Tác DOM Thủ Công
- **Vấn đề:** Frontend viết bằng Vanilla JS không sử dụng framework phản ứng (Reactive State Management). Mọi thao tác cập nhật dữ liệu đều dựa vào `document.getElementById(...)`, gán innerHTML trực tiếp và gắn hàm vào biến toàn cục `window.*`.
- **Hệ quả:** Codebase rất khó bảo trì, dễ phát sinh lỗi rò rỉ bộ nhớ (memory leaks) khi thêm/xóa DOM, và tiềm ẩn nguy cơ bất đồng bộ trạng thái khi có nhiều sự kiện diễn ra cùng lúc.

### 6.7 Trói Buộc Chặt Chẽ Vào Hệ Điều Hành Windows (Platform Coupling)
- **Vấn đề:** Toàn bộ codebase backend phụ thuộc vào các thư viện liên kết động của Windows:
  - `syscall.NewLazyDLL("kernel32.dll")`
  - `syscall.NewLazyDLL("user32.dll")`
  - Lệnh `rundll32 url.dll,FileProtocolHandler`
  - Biến môi trường `%APPDATA%`, `%LOCALAPPDATA%`, `%ProgramFiles%`
  - Cờ tiến trình Windows `syscall.SysProcAttr{HideWindow: true, CreationFlags: 0x08000000}`
- **Hệ quả:** Không thể biên dịch và chạy trên macOS hoặc Linux mà không phải viết lại hoàn toàn lớp trừu tượng hệ điều hành (OS abstraction layer).

---

## 7. Đánh Giá Toàn Diện Lỗ Hổng Bảo Mật (Security Vulnerability Assessment)

| Cấp Độ | Mã Lỗ Hổng | Tên Lỗ Hổng | Tệp Tin Bị Ảnh Hưởng | Mô Tả & Tác Động |
| :--- | :--- | :--- | :--- | :--- |
| **CRITICAL** | **SEC-01** | Lưu trữ bí mật ở dạng Plaintext không mã hóa | `config/config.go`, `.env` | GitHub PAT (`ghp_...`) và AI API Key (`sk-xt-...`) được lưu trữ hoàn toàn dưới dạng văn bản thô không mã hóa trong `%APPDATA%\GGitUpload\config.json` và file `.env`. Bất kỳ ứng dụng nào hoặc người dùng nào có quyền đọc thư mục người dùng đều có thể đánh cắp toàn bộ quyền hạn truy cập tài khoản GitHub. |
| **CRITICAL** | **SEC-02** | Lộ Token nhạy cảm thực tế trong mã nguồn dự án | `.env` | Tệp `.env` trong thư mục gốc dự án chứa token thật còn hiệu lực (`GITHUB_TOKEN`, `GITHUB_TOKEN_FULL_ACCESS`, `AI_API_KEY`) thay vì giá trị mẫu. |
| **HIGH** | **SEC-03** | Lộ Token xác thực qua Tham số Dòng Lệnh Git | `push_service.go`, `clone_service.go`, `git/runner.go` | Remote URL được tạo thành `https://{token}@github.com/...`. Khi chạy lệnh `git remote add origin` hoặc `git clone`, URL chứa token này trở thành tham số dòng lệnh của process hệ thống. Bất kỳ phần mềm nào trên Windows có thể đọc danh sách tiến trình (Process Explorer, Task Manager, malware) đều có thể lấy cắp token này trong lúc lệnh đang chạy. |
| **HIGH** | **SEC-04** | Đẩy đè mã nguồn cưỡng bức không thể hoàn tác | `git/runner.go` (L184) | Lệnh `git push --force` thực hiện ghi đè toàn bộ lịch sử git trên remote mà không hề có hộp thoại cảnh báo hay kiểm tra tính toàn vẹn của nhánh đích. |
| **MEDIUM** | **SEC-05** | Nguy cơ XSS/DOM Injection từ nội dung README từ xa | `search_service.go`, `frontend/src/services/SearchService.js` | Hàm `loadRepoReadme` lấy mã HTML thô từ GitHub API (`Accept: application/vnd.github.html`) và gán thẳng vào thuộc tính `readmeContent.innerHTML = html` mà không đi qua bộ lọc làm sạch (Sanitizer như DOMPurify). Nếu repository chứa mã độc hại, WebView2 có thể bị khai thác. |
| **MEDIUM** | **SEC-06** | Tải nội dung script/hướng dẫn từ URL công khai không kiểm định | `system_service.go` (L127) | `GetLocalGuideHTML` tải văn bản từ GitHub raw URL không có chữ ký số xác thực (checksum/signature). Nếu repo bị tấn công hoặc đường dẫn bị chiếm quyền kiểm soát (DNS spoofing/MITM), nội dung độc hại sẽ được nạp trực tiếp vào ứng dụng. |
| **LOW-MED** | **SEC-07** | Thiếu Rate Limiting & Rủi ro Prompt Injection tại AI Service | `ai_service.go`, `frontend/src/services/ChatService.js` | Gửi trực tiếp log console và input người dùng lên API OpenRouter mà không giới hạn độ dài chặt chẽ, không có cơ chế cắt giảm chi phí token, và dễ bị Prompt Injection để vượt qua quy tắc của Stephen AI. |

---

## 8. Đối Chiếu Đặc Tả SRS v2.0 & Định Hướng Viết Lại Bằng Rust (Gap Analysis & Rust Roadmap)

### 8.1 Ma Trận Đối Chiếu Khoảng Cách Tính Năng (SRS v2.0 Gap Matrix)

| Hệ Thống Tính Năng (Epic) | Yêu Cầu Tại SRS v2.0 | Tình Trạng Trong Bản Cũ (v1.4.1 Go) | Đánh Giá Mức Độ Thiếu Hụt & Yêu Cầu Thiết Kế Lại |
| :--- | :--- | :--- | :--- |
| **Epic 1: Multi-Profiles & Multi-Platforms** | Quản lý nhiều Profile (GitHub cá nhân, GitHub tổ chức, GitLab); Chuyển đổi Profile thời gian thực; Interface `VCSProvider` theo Strategy Pattern. | **HOÀN TOÀN CHƯA CÓ**. Chỉ hỗ trợ 1 tài khoản GitHub duy nhất, code gắn chặt vào API của GitHub (`github/client.go`), không hỗ trợ GitLab hay Gitea. | **Phải xây dựng mới 100%**. Cần thiết kế `trait VcsProvider` trong Rust và cấu trúc lại schema lưu trữ profile. |
| **Epic 2: AI-Powered Workflow** | Tự động sinh Smart Commit từ `git diff`; Quét bảo mật mã nguồn (AI Code Review); Tự động tạo `README.md` trước khi push; Thuật toán phân đoạn Diff (Diff Chunking). | **CHƯA ĐẠT**. Chỉ có khung chat hỏi đáp thông thường (Stephen AI); không có chức năng đọc `git diff` để sinh commit; không có review mã nguồn; không tự động tạo file README. | **Phải thiết kế lại toàn bộ luồng AI**. Xây dựng cơ chế trích xuất diff từ Git engine và tích hợp prompt chuẩn hóa đầu ra JSON. |
| **Epic 3: Multi-file Upload & Smart `.gitignore`** | Nhận mảng nhiều đường dẫn file; Gom file vào thư mục tạm với Buffer I/O tối ưu; Tự động nhận diện ngôn ngữ (Node, Java, Go, Python); Tự động chèn mẫu `.gitignore` chuẩn. | **CHƯA ĐẠT**. Bản v1.4.1 chỉ hỗ trợ đúng 1 file lẻ duy nhất; hàm `.gitignore` chỉ ghi cứng mỗi chữ `.env`; không có cơ chế nhận diện cấu trúc dự án (`pom.xml`, `package.json`, `go.mod`). | **Cần xây dựng Engine phân tích dự án & thư viện template `.gitignore`**. Chuyển sang xử lý mảng đường dẫn file `Vec<PathBuf>`. |
| **Epic 4: Analytics Dashboard** | Thống kê học tập trực quan; Heatmap đóng góp tương tự GitHub; Biểu đồ tỷ lệ Session vs MiniProject; Lưu trữ lịch sử chi tiết có đánh chỉ mục. | **HOÀN TOÀN CHƯA CÓ**. `history.json` chỉ lưu 1 bản ghi duy nhất của lần push cuối cùng. Không có biểu đồ, không có bộ lọc thời gian. | **Phải thay thế cơ chế lưu trữ JSON bằng Cơ sở dữ liệu nhúng (SQLite)** và tích hợp thư viện biểu đồ phía giao diện. |

---

### 8.2 Khuyến Nghị Chiến Lược Cho Bản Viết Lại Bằng Rust (`rust-gitupload`)

Để giải quyết triệt để toàn bộ các nút thắt hiệu năng, nợ kỹ thuật và lỗ hổng bảo mật được phát hiện trong đợt kiểm toán này, phiên bản viết lại bằng ngôn ngữ Rust cần tuân thủ các nguyên tắc thiết kế kỹ thuật sau:

```
┌────────────────────────────────────────────────────────────────────────┐
│                   GIAO DIỆN NGƯỜI DÙNG (UI LAYER)                      │
│             Tauri v2 Core + WebView2 / Svelte / React / Slint          │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ IPC (Tauri Commands & Events)
┌───────────────────────────────────▼────────────────────────────────────┐
│                  LỚP NGHIỆP VỤ CỐT LÕI (RUST CORE)                     │
│                                                                        │
│  ┌──────────────────────┐  ┌────────────────────┐  ┌────────────────┐  │
│  │  Profile & Security  │  │   VCS Abstraction  │  │   Git Engine   │  │
│  │  Manager (Keyring)   │  │ (trait VcsProvider)│  │ (git2-rs / gix)│  │
│  └──────────┬───────────┘  └──────────┬─────────┘  └────────┬───────┘  │
│             │                         │                     │          │
│  ┌──────────▼───────────┐  ┌──────────▼─────────┐  ┌────────▼───────┐  │
│  │   Local Database     │  │   AI Orchestrator  │  │ File & Project │  │
│  │   (rusqlite / SQL)   │  │   (Diff Chunking)  │  │   Inspector    │  │
│  └──────────────────────┘  └────────────────────┘  └────────────────┘  │
└────────────────────────────────────────────────────────────────────────┘
```

#### 1. Thay Thế Git CLI Bằng Thư Viện Git Nhúng Của Rust
- **Khuyến nghị:** Sử dụng crate **`git2-rs`** (liên kết trực tiếp `libgit2` đã được kiểm chứng về độ ổn định) hoặc crate **`gix`** (viết bằng 100% Rust an toàn).
- **Lợi ích:** 
  - Loại bỏ hoàn toàn sự phụ thuộc vào `git.exe` trên hệ điều hành của học viên.
  - Tốc độ khởi tạo repo, add, commit và push diễn ra trên cùng tiến trình trong bộ nhớ, nhanh hơn gấp 10-20 lần so với việc liên tục tạo subprocess trên Windows.
  - Dễ dàng bắt các callback tiến độ mạng (`RemoteCallbacks::transfer_progress`) với độ chính xác tuyệt đối mà không cần dùng regex phân tích chuỗi stdout.

#### 2. Bảo Mật Bí Mật Với OS Credential Manager (Khắc phục triệt để SEC-01 & SEC-03)
- **Khuyến nghị:** Sử dụng crate **`keyring-rs`** để tương tác với **Windows Credential Manager** (hoặc macOS Keychain / Linux Secret Service).
- **Lợi ích:** Token GitHub/GitLab và AI Key được mã hóa bằng cơ chế bảo mật cấp hệ điều hành (DPAPI trên Windows). File cấu hình `config.json` chỉ lưu ID tham chiếu, không bao giờ chứa token dạng thô. Khi truyền credential cho Git, sử dụng in-memory memory credentials helper thay vì đưa token vào URL dòng lệnh.

#### 3. Chuyển Đổi Sang Cơ Sở Dữ Liệu Nhúng SQLite (Khắc phục hạn chế 6.1)
- **Khuyến nghị:** Sử dụng crate **`rusqlite`** hoặc **`sqlx`** với driver SQLite nhúng.
- **Lợi ích:** 
  - Lưu trữ không giới hạn toàn bộ lịch sử các lần đẩy bài tập cùng metadata phong phú (`id`, `profile_id`, `repo_name`, `repo_url`, `commit_msg`, `mode`, `session_num`, `exercise_num`, `file_count`, `duration_ms`, `pushed_at`).
  - Dễ dàng thực thi các truy vấn SQL tính toán số liệu thống kê học tập, nhóm theo ngày/tuần để vẽ biểu đồ Heatmap cho Epic 4 một cách tức thì mà không cần nạp toàn bộ file JSON khổng lồ vào RAM.

#### 4. Thiết Kế Trừu Tượng Đa Nền Tảng Git (Strategy Pattern)
- **Khuyến nghị:** Định nghĩa trait bất đồng bộ trong Rust:
  ```rust
  #[async_trait]
  pub trait VcsProvider: Send + Sync {
      async fn get_user_profile(&self) -> Result<UserProfile, VcsError>;
      async fn create_repository(&self, req: &CreateRepoRequest) -> Result<Repository, VcsError>;
      async fn check_repository_empty(&self, repo_name: &str) -> Result<bool, VcsError>;
      async fn search_repositories(&self, query: &str, page: usize) -> Result<RepoSearchResult, VcsError>;
      async fn delete_repository(&self, repo_name: &str) -> Result<(), VcsError>;
  }
  ```
- **Lợi ích:** Dễ dàng triển khai `GitHubProvider`, `GitLabProvider`, và `GiteaProvider` độc lập, hỗ trợ chuyển đổi linh hoạt tài khoản cá nhân / tổ chức theo đúng thiết kế của SRS v2.0.

#### 5. Khung Ứng Dụng Desktop Thế Hệ Mới
- **Khuyến nghị:** Sử dụng **Tauri v2**.
- **Lợi ích:** 
  - Dung lượng file thực thi nhỏ gọn (~5-8 MB so với Wails/Go ~15-20 MB và Electron ~100 MB).
  - Khả năng kiểm soát bộ nhớ chặt chẽ của Rust (không có Garbage Collector gây giật lag hoặc spike RAM ngẫu nhiên).
  - Tích hợp sẵn kiến trúc bảo mật đa lớp (Security Scopes, CSP, Path Validation) ngăn chặn triệt để các lỗ hổng XSS và Command Injection.

---
*Tài liệu kiểm toán này phản ánh chính xác 100% hiện trạng mã nguồn cũ của dự án `g-gitupload` (v1.4.1) và đóng vai trò làm tài liệu kỹ thuật nền tảng cho đội ngũ Kiến trúc sư (`@architect`) và Tác giả SRS (`@srs_author`) triển khai xây dựng phiên bản Rust v2.0.*

# Hướng Dẫn Sử Dụng GitUpload Desktop v2.2

Chào mừng bạn đến với **GitUpload Desktop Suite (v2.2)** — phiên bản hiện đại hóa toàn diện trên nền tảng **Rust + Tauri v2 + GitHub Primer Design System**.

---

## 1. Cấu Hình Tài Khoản & Bảo Mật Keyring

1. **GitHub Personal Access Token (PAT) hoặc Dual Auth Bridge (`gh`)**:
   - Nếu bạn đã đăng nhập `gh auth login`, hãy nhấn **Sync from GitHub CLI** trong tab *Settings* để đồng bộ token tự động vào **Windows Credential Manager**.
   - Nếu dùng PAT thủ công (`ghp_...` hoặc `github_pat_...`), dán trực tiếp trong *Settings*. Token được mã hóa cấp hệ điều hành (DPAPI) — tuyệt đối không ghi ra tệp JSON hay `.env`.
2. **GitHub Organizations**:
   - Nhập tên Organization hoặc chọn trực tiếp từ danh sách Organization của bạn để tạo và đẩy repository vào tổ chức.
3. **Pure Cloud AI**:
   - Hỗ trợ **Google Gemini 2.0/1.5**, **OpenAI GPT-4o**, **Anthropic Claude 3.5**, **Groq**, và **OpenRouter** qua SSE streaming siêu nhẹ (**< 28 MB RAM**).

---

## 2. Các Tính Năng Chính

- **Upload & Sync (Folder & Single File Isolation)**:
  - Hỗ trợ kéo thả thư mục hoặc 1 file lẻ (`SS15_HW02_Ten_MaSV.md`) với chế độ cô lập thư mục tạm sạch sẽ.
  - Hỗ trợ **Re-Push** trên thư mục đã có `.git`, hiển thị số commit *Ahead / Behind*, Auto-Stash và **Safe Sync (`--force-with-lease`)**.
- **Interactive Git Graph (DAG)**:
  - Hiển thị cây lịch sử commit nhiều làn bơi màu, hỗ trợ **Cherry-pick**, **Revert**, **Reset (Soft/Mixed/Hard)**, tạo Branch/Tag trực tiếp.
- **Native Rust Git Hooks (`gitupload-hooks`)**:
  - Kiểm tra **Conventional Commits 1.0.0**, chặn tệp `.env`, cảnh báo tệp > 50MB (Git LFS), và quét lộ secret trước khi commit mà không cần Node.js.
- **Pull Requests, GitHub Actions, Issues & Releases**:
  - Tạo Draft/Standard PR, duyệt và merge (`Squash`, `Rebase`, `Merge`), theo dõi log GitHub Actions theo thời gian thực, quản lý Issues và tạo Release kèm AI Changelog.

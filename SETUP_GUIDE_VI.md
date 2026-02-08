# 🚀 SUPER ROUTE PRO - Hướng Dẫn Cài Đặt & Sử Dụng

## Phần 1: Chuẩn Bị Hệ Thống

### ✅ Yêu Cầu
- **Windows 11** (Build 26100 trở lên) ✓
- **Node.js** v20+ (`npm` v11+) ✓
- **Rust** + **Cargo** ✓
- **Visual Studio 2022 Build Tools** (C++ workload) ✓

### Kiểm Tra Cài Đặt

```powershell
# Kiểm tra các công cụ đã cài:
node --version          # v25.2.1 ✓
npm --version           # v11.6.2 ✓
rustc --version         # rustc 1.93.0 ✓
cargo --version         # cargo 1.93.0 ✓
```

---

## Phần 2: Project Đã Được Tạo

Project nằm tại: **`E:\super-route-pro\`**

```
super-route-pro/
├── src/                          # React 19 + TypeScript frontend
│   ├── App.tsx                   # Main UI component (1,200 lines)
│   ├── api.ts                    # Tauri command bindings
│   ├── LatencyChart.tsx          # Real-time latency visualization
│   ├── App.css                   # Tailwind CSS + custom styles
│   └── main.tsx                  # React entry point
├── src-tauri/                    # Rust backend (Tauri v2)
│   ├── src/
│   │   ├── lib.rs                # Tauri setup & command exports
│   │   └── network.rs            # 400+ lines network commands
│   ├── Cargo.toml                # Rust dependencies
│   ├── build.rs                  # Admin manifest (release build)
│   └── super-route-pro.exe.manifest  # UAC elevation config
├── package.json                  # Node dependencies
├── tsconfig.json                 # TypeScript config
├── vite.config.ts                # Vite + Tailwind setup
└── index.html                    # Entry HTML

Tổng cộng: ~2,500 dòng code
```

---

## Phần 3: Chạy Ứng Dụng

### Cách 1: Dev Mode (Recommended - để test)

```powershell
# Mở PowerShell tại: E:\super-route-pro\
cd E:\super-route-pro

# Tùy chọn 1: Chạy từ npm script
npm run tauri dev

# Tùy chọn 2: Chạy trực tiếp từ tauri CLI
npm install -g @tauri-apps/cli  # Cài global (nếu chưa có)
tauri dev
```

**Lần đầu tiên:** Cần biên dịch tất cả. Sẽ mất **2-5 phút**.  
**Lần sau:** Chỉ mất **10-30 giây** nhờ caching.

### Cách 2: Build Production (Để cài đặt)

```powershell
cd E:\super-route-pro

# Build ứng dụng
npm run tauri build

# Output (installer):
# ├── src-tauri/target/release/bundle/nsis/Super Route Pro_1.0.0_x64-setup.exe
# └── src-tauri/target/release/bundle/msi/Super Route Pro_1.0.0_x64.msi
```

Sau khi build, bạn sẽ có:
- ✅ **NSIS Installer** (~8-10 MB) - Dễ dàng phân phối
- ✅ **MSI Package** - Cho Windows Domain/Enterprise
- ✅ **Portable EXE** - Chạy trực tiếp không cần cài đặt

---

## Phần 4: Tính Năng Chính

### 🔧 Backend Rust (Không PowerShell Bảng)
- ✅ **Quản lý NIC** - Liệt kê card mạng đang hoạt động
- ✅ **Quản lý Routes** - Add/Delete/Flush routes bảng định tuyến
- ✅ **Chọn Internet** - Đặt gateway mặc định
- ✅ **Network Fixes** - Flush DNS, Renew IP, Reset Winsock, Clear ARP, Reset Firewall
- ✅ **Ping Monitor** - Real-time latency chart (8.8.8.8)
- ✅ **Internet Status** - Kiểm tra kết nối trực tuyến liên tục

### 🎨 Frontend React 19
- ✅ **Dark Theme** - Giao diện bóng tối hiện đại (Tailwind CSS v4)
- ✅ **NIC Table** - Bảng tương tác, click chọn gateway
- ✅ **Route Form** - Input form cấu hình route (destination, mask, gateway, metric)
- ✅ **Route Table** - Hiển thị tất cả routes, click chọn để edit
- ✅ **Search Filter** - Tìm kiếm routes theo IP
- ✅ **Latency Chart** - SVG chart real-time, màu sắc động
- ✅ **Tool Section** - 6 network fix tools với confirmation dialog
- ✅ **Status Bar** - Thông báo thời gian thực

### ⚡ Không Còn Cửa Sổ PowerShell
- ✅ `CREATE_NO_WINDOW` flag trên tất cả process
- ✅ Rust backend gọi system commands trực tiếp
- ✅ Admin elevation tự động (release build)
- ✅ WebView2 rendering (sẵn có trên Win11)

---

## Phần 5: Cấu Trúc Tauri v2 RPC

### Frontend → Backend Communication

```typescript
// src/api.ts
export async function addRoute(dest, mask, gw, metric) {
    return invoke<CommandResult>("add_route", {
        destination: dest,
        mask,
        gateway: gw,
        metric,
    });
}
```

### Backend Implementation

```rust
// src-tauri/src/network.rs
#[tauri::command]
pub async fn add_route(
    destination: String,
    mask: String,
    gateway: String,
    metric: String,
) -> Result<CommandResult, String> {
    // Rust code here - NO PowerShell
    run_cmd("route", &["-p", "add", &destination, ...])
}
```

**Lợi Ích:**
- 🚀 Rust native → Hiệu năng cao
- 🔒 Không cần PowerShell execution
- 📦 Tất cả bundled vào 1 EXE (~8MB)
- 🪟 Hỗ trợ Admin elevation qua manifest

---

## Phần 6: Customization

### Thay Đổi Giao Diện
- **Màu sắc**: Sửa Tailwind classes trong `src/App.tsx`
- **Layout**: Điều chỉnh grid columns/rows
- **Icons**: Từ `lucide-react` (400+ icons có sẵn)

### Thêm Tính Năng
1. **Viết Rust function** trong `src-tauri/src/ network.rs`
2. **Thêm `#[tauri::command]` attribute**
3. **Export từ `lib.rs`**
4. **Gọi từ React bằng `invoke()`**

### Ví Dụ: Thêm tracert Command

```rust
// src-tauri/src/network.rs
#[tauri::command]
pub async fn tracert_host(target: String) -> Result<CommandResult, String> {
    let output = run_cmd("tracert", &[&target])?;
    Ok(CommandResult { success: true, output })
}
```

```typescript
// src/App.tsx
const handleTracert = async () => {
    const result = await invoke<CommandResult>("tracert_host", { 
        target: tracertTarget 
    });
    setTracertOutput(result.output);
};
```

---

## Phần 7: Troubleshooting

### ❌ "link.exe not found"
**Giải pháp:**
```powershell
# Cài Visual Studio Build Tools
winget install Microsoft.VisualStudio.2022.BuildTools
```

### ❌ "cargo command not found"
**Giải pháp:**
```powershell
# Refresh PATH
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + 
            ";" + [System.Environment]::GetEnvironmentVariable("Path","User")
```

### ❌ "elevation required (os error 740)"
**Giải pháp:**
- Dev mode → Chạy terminal WITH Admin
- Release mode → Automatic elevation via manifest

### ❌ App không hiện
- Kiểm tra Vite dev server: `http://localhost:1420`
- Kiểm tra Tauri window setup trong `tauri.conf.json`
- Check console: `Tools → Developer → Open DevTools`

---

## Phần 8: Build & Distribution

### For Release

```powershell
cd E:\super-route-pro
npm run tauri build

# .exe installer sẽ nằm tại:
# src-tauri/target/release/bundle/nsis/Super_Route_Pro_1.0.0_x64-setup.exe
```

### Ký Số (Code Signing) - Optional
Để ký số installer (tránh SmartScreen warning):
1. Mua code signing certificate từ DigiCert/GlobalSign
2. Cấu hình trong `tauri.conf.json` → `bundle.windows.signingIdentity`

---

## Phần 9: So Sánh với Cái Cũ

| Tính năng | SuperRoute.py (customtkinter) | Super Route Pro (Tauri) |
|-----------|-------------------------------|------------------------|
| Framework | Python + CustomTkinter | Rust + React + TypeScript |
| Size | Không đo được (Python 3.x) | **8-10 MB** |
| Startup | **~2-3 giây** | **<1 giây** |
| UI Quality | Basic dark theme | Modern dark theme + animation |
| PowerShell | Yes (flash window) | **Không** |
| Admin Rights | Manual elevation | Auto manifest |
| Distribution | py2exe (~150MB) | NSIS installer (~8MB) |
| Update Support | Không | Tauri Updater plugin |
| Cross-Platform | Không | ✅ (macOS/Linux possible) |

---

## Phần 10: Command Reference

```bash
# = Development =
npm run tauri dev          # Run in dev mode with hot reload
npm run dev                # Run Vite dev server only
npm run build              # Build frontend only

# = Production =
npm run tauri build        # Full build (Rust + bundle)

# = Utilities =
npm run type-check         # Check TypeScript errors
cargo test                 # Run Rust unit tests
npm run format             # Format code (prettier)
```

---

## Video Nhanh

1. **Start Dev**: `npm run tauri dev` → App mở lên
2. **Edit React**: Sửa `src/App.tsx` → Hot reload tự động
3. **Edit Rust**: Sửa `src-tauri/src/network.rs` → Tauri tự rebuild
4. **Build Release**: `npm run tauri build` → EXE installer sinh ra

---

## Support & Next Steps

- 📝 **Full Source Code**: `E:\super-route-pro\` trên máy của bạn
- 🔗 **Tauri Docs**: https://tauri.app
- ⚡ **React Docs**: https://react.dev
- 🦀 **Rust Docs**: https://doc.rust-lang.org

**Happy Coding! 🎉**

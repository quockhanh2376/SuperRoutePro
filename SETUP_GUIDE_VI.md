# SUPER ROUTE PRO - Huong Dan Cai Dat Va Test

## 1. Yeu cau

- Windows 10/11 x64
- Node.js 20+
- npm 10+
- Rust toolchain (`rustup`, `cargo`)
- Visual Studio 2022 Build Tools (C++ workload)

## 2. Mo hinh quyen moi

- UI cua app chay voi quyen `standard user`.
- App khong con auto-elevate toan bo process luc startup.
- Khi IT can chay tac vu he thong, app mo `Repair Mode`.
- Windows chi xin credential admin local mot lan cho moi phien app.
- Khi dong app, repair session se bi khoa lai.

## 3. Startup va WebView2

- Installer release van bundle WebView2 `offlineInstaller`.
- Luc startup, app tu tao `main-webview` duoi `LocalAppData`.
- Neu WebView2 bao data directory loi, app se:
  - rotate thu muc cu sang `main-webview-reset*`
  - tao thu muc moi
  - thu build lai mot lan

## 4. Development

```powershell
npm ci
npm run tauri dev
```

Check nhanh truoc khi build:

```powershell
npm run check
cargo test --manifest-path src-tauri/Cargo.toml
```

## 5. Build release local

```powershell
npm run release:local
```

Artifacts duoc tao trong `release-artifacts/vX.Y.Z/`:

- `Super Route Pro_<version>_x64-setup.exe`
- `SuperRoute.exe`
- `SHA256SUMS.txt`

## 6. Cai tren may test standard user

1. IT download installer tu GitHub Releases.
2. IT chay installer bang quyen admin.
3. Dang nhap vao Windows bang user standard.
4. Mo app, UI phai len binh thuong va khong duoc hien UAC luc startup.
5. Chon `Target User` neu can cleanup profile/Appx.
6. Bam `Unlock Repair Mode` khi can tac vu admin.
7. Windows se hien prompt de IT nhap local admin credential.
8. Sau khi unlock, route/reset/cleanup/Appx chay qua elevated repair broker cho den khi dong app.

## 7. Checklist test Win11

- App mo duoc bang standard user, khong popup admin luc startup.
- Neu `main-webview` stale/corrupt, app van vao duoc UI sau 1 lan tu recovery.
- `Repair Mode` mac dinh la `Locked`.
- Route add/delete/flush va network reset bi khoa khi chua unlock.
- Chon `Target User` truoc khi `Clear Cache` hoac `Remove Apps`.
- Unlock thanh cong thi cac tac vu admin chay duoc.
- Dong app xong mo lai thi `Repair Mode` ve `Locked`.

## 8. Ghi chu release

- Release `9.0.2` dong goi elevated helper `SuperRouteRepairBroker`.
- Installer khong con co gang cai placeholder Windows service.
- Neu IT cancel UAC luc unlock, app van giu UI o standard-user mode va bao loi unlock.

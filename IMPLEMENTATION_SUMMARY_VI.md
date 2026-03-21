# SUPER ROUTE PRO - Implementation Summary (Cap nhat 2026-03-14)

## Muc tieu

Sua triet de 2 van de chinh:

- App release khong mo duoc o may Win11 dang dang nhap bang standard user.
- Cac tac vu admin dang elevate toan bo app thay vi chi elevate luc can fix he thong.

## Ket qua chinh

### 1. UI startup an toan cho standard user

- Manifest UI giu `asInvoker`.
- Code startup khong con relaunch toan bo app bang admin.
- App van check Windows/WebView2/runtime prerequisites.

### 2. WebView2 data directory co self-heal

- `main-webview` duoc preflight write test truoc khi build webview.
- Neu WebView2 bao data directory loi:
  - thu muc cu duoc rotate sang `main-webview-reset*`
  - app tao thu muc moi
  - retry build 1 lan
- Neu rotate that bai, app co the fallback sang `main-webview-recovery*`.

### 3. Repair Mode theo phien

- Frontend van giu `Repair Mode: Locked/Unlocked`.
- Unlock khong doi toan bo UI sang admin context.
- App launch `SuperRouteRepairBroker` khi IT unlock.
- Broker duoc elevate qua UAC va giu session repair cho den khi app dong/lock.

### 4. Privileged actions da di qua elevated helper

- Add/Delete/Flush route
- Set default gateway
- Persist WAN startup task
- Flush DNS / Renew DHCP / Clear ARP / Reset TCP-IP / Reset Winsock / Reset Firewall / Reset WinHTTP Proxy / Restart adapters
- Profile cleanup theo `target_sid`
- Appx removal theo `target_sid` + optional `remove_provisioned`

## IPC va bao mat

- UI va broker noi qua loopback TCP local.
- Moi request privileged deu mang `auth_token` ngau nhien cua session unlock.
- Neu helper mat ket noi, UI quay ve trang thai `Locked`.
- Dong app se goi lock/shutdown de ket thuc repair session.

## Packaging release

- Bundle chi stage `SuperRouteRepairBroker` sidecar.
- NSIS van la duong installer chinh.
- Khong con installer hook `sc create` cho placeholder service.
- Release artifact local:
  - NSIS setup
  - portable `SuperRoute.exe`
  - `SHA256SUMS.txt`

## Test va verification da chay

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
npm run check
cargo build --manifest-path src-tauri/Cargo.toml --bin SuperRouteRepairBroker
```

Smoke test IPC da chay local:

- Start `SuperRouteRepairBroker --serve <port> <token> <app> <conn>`
- Gui `GetServiceHealth`
- Gui `GetRepairSessionStatus`
- Gui `Shutdown`
- Ket qua: host tra ve `connected=true`, session `locked=false`, va shutdown tra ve session `locked=true`

## Gioi han con lai

- Chua co bai test end-to-end thay the mot may Win11 that dang login bang standard user va unlock bang local admin credential.
- Dieu nay can IT/QA chay tren may test that truoc khi GitHub release `10.1.0` duoc publish.

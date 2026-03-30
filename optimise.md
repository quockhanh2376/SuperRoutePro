# Tối ưu hoá kỹ thuật và lộ trình v10.1.9

## Kết luận ngắn
- `CHANGELOG.md` cho thấy v10.1.9 nghiêng mạnh về polish giao diện Speed Test. Đây là bước đúng cho UX, nhưng nền đo đạc và contract catalog vẫn cần siết nếu muốn hệ thống bền và mở rộng tiếp.
- AU hiện vẫn nên được xem là auto-region, chưa phải city-pinned. Điều này đã được ghi rõ trong `NeedToDo.md` và `CHANGELOG.md`.
- Có lệch single source of truth giữa backend và UI/test: catalog ở `src-tauri/src/speed_test_targets.rs` và `src-tauri/tests/speed_test_targets_contract.rs` đã là Auto Asia và Cloudflare Asia auto-edge, nhưng fallback và fixture ở `src/SpeedTestModal.tsx` và `tests/SpeedTestModal.test.tsx` vẫn còn string cũ.

## 1. Tối ưu và fix để codebase bền hơn
- P0: Tách `src/App.tsx` thành các shell theo feature. File này đã vượt mốc 2.1k dòng và đang giữ quá nhiều state UI, diagnostics, routing, repair và speed test trong cùng một component. Tối thiểu tách thành Network, Diagnostics, Repair, SpeedTest và Shared Modals.
- P0: Chuẩn hoá target catalog thành một nguồn dữ liệu dùng chung cho Rust, UI fallback, browser demo và test. Không để label, provider, description bị lặp bằng tay ở nhiều nơi.
- P1: Tách `src-tauri/src/network.rs` theo domain. Hiện file này đang ôm route CRUD, DHCP renew, command guard, adapter actions, bloatware và connectivity probe. Nên chia thành `route_ops`, `diag_ops`, `adapter_ops`, `command_guard` và `queries`.
- P1: Tách `src-tauri/src/repair_ipc.rs` theo lớp. Framing TCP, session state, unlock handshake và command dispatch nên thành các module độc lập để dễ test và dễ cô lập lỗi.
- P1: Tách `src-tauri/src/speed_test.rs` thành transport/provider adapters, measurement math, preflight context và result mapping. Mục tiêu là thêm target mới mà không chạm cả file.
- P1: Mở rộng schema ở `src/api.ts` sớm với `selection_kind`, `backend_kind`, `route_fit`, `resolved_colo`, `payload_profile`, `city` và `country`. Làm sớm sẽ tránh breaking change khi bước sang AU city-based.

## 2. Việc phải làm ngay để tăng độ chính xác Speed Test
- Now: Đổi cách đo latency trong `src-tauri/src/speed_test.rs`. Hiện engine lấy 6 mẫu, chấp nhận 3 mẫu thành công và dùng trung bình đơn giản. Nên đổi sang 8 đến 10 mẫu, bỏ 1 mẫu warm-up, yêu cầu ít nhất 5 mẫu hợp lệ và trả về median ping.
- Now: Đổi cách tính jitter ở `src-tauri/src/speed_test.rs`. Jitter hiện là trung bình độ lệch tuyệt đối giữa các RTT liên tiếp, quá nhạy với outlier. Nên tính trên tập mẫu đã lọc warm-up và outlier, ưu tiên median-based hoặc trimmed mean.
- Now: Bỏ mô hình payload cố định làm chuẩn so sánh xuyên vùng. `src-tauri/src/speed_test_targets.rs` đang dùng Auto Asia 24 MB, Auto Australia 20 MB, JP/KR 4 MB, US West 4 MB, EU 1 MB. Nên đổi sang chiến lược duration-based: bắt đầu nhỏ, tăng dần đến khi đạt 4 đến 6 giây hoặc chạm trần an toàn của target.
- Now: Giữ guard rõ ràng cho Auto Australia. `CHANGELOG.md` cho thấy đường 16 MB từng trả HTTP 403; khi chuyển sang adaptive payload phải có blacklist hoặc profile guard cho size xấu, không chỉ đổi default.
- Now: Tách timeout theo stage. Latency cần timeout ngắn hơn nhiều so với download và upload; dùng chung một budget 90 giây sẽ che mờ lỗi và làm kết quả probe kém sắc.
- Now: Đưa `route_fit` và `resolved_colo` ra contract kết quả. `src-tauri/src/speed_test.rs` đã phân biệt PreferredRegion và GlobalFallback, nhưng UI chưa dùng đủ để cảnh báo người dùng rằng run auto-edge đang lệch vùng ưu tiên.
- Now: Trả thêm telemetry tối thiểu gồm `latency_samples`, `successful_latency_samples`, `download_bytes`, `upload_bytes`, `elapsed_ms`, `resolved_colo` và `route_fit`. Không nhất thiết phải hiển thị hết, nhưng phải có để debug accuracy và theo dõi regression.
- Now: Thêm seam test cho measurement engine. Test hiện tại ở `tests/SpeedTestModal.test.tsx`, `src-tauri/tests/speed_test_targets_contract.rs` và unit test trong `src-tauri/src/speed_test.rs` mới khoá label, request builder và route fit; chưa khoá được latency filtering, adaptive payload, HTTP 403 fallback hay throughput math một cách deterministic.

## 3. Chuẩn bị kiến trúc cho phase AU city-based
- Gate 1: Chưa mở selector Sydney, Melbourne, Brisbane, Perth ở release gần nhất. Chỉ nên làm sau khi có backend city-pinned thật, ổn định và đã qua runtime qualification như ghi trong `NeedToDo.md`.
- Gate 1: Nâng model target ở `src-tauri/src/speed_test_targets.rs` để tách rõ `auto_region`, `fixed_region` và `fixed_city`. Không để UI phải suy luận từ label.
- Gate 1: Thay branching cứng theo backend kind bằng provider adapter. Cloudflare auto-edge và LibreSpeed fixed backend đã khác nhau đáng kể; AU city-based gần như chắc chắn sẽ cần adapter hoặc policy thứ ba.
- Gate 2: Chuyển catalog từ const array thuần sang catalog có metadata mở rộng và health policy. Vẫn có thể giữ typed Rust struct, nhưng cần một lớp manifest nội bộ để thêm target mới mà không sửa measurement logic.
- Gate 2: Thêm pipeline qualification cho endpoint AU trước khi expose lên UI: reachability, IP lookup format, payload stability, expected city identity, median latency floor và error rate chấp nhận được.
- Gate 2: Thiết kế contract UI theo các cột Region, City, Provider, Pinning mode, Route fit và Confidence để người dùng phân biệt city-pinned verified với nearest AU edge.
- Gate 3: Giữ `PREFERRED_AU_COLOS` như heuristic vùng, không coi đây là city pinning. Danh sách `SYD`, `MEL`, `BNE`, `PER`, `ADL` chỉ đủ để xác nhận edge thuộc Australia, không đủ để tuyên bố test đang pin vào một thành phố cố định.

## Lộ trình đề xuất
- 1 tuần: hợp nhất catalog dùng chung cho backend, UI, demo và test; sửa fixture lệch contract; thêm `route_fit`, `resolved_colo` và telemetry cơ bản vào kết quả.
- 1 đến 2 release kế tiếp: tách component gốc và các module backend lớn; triển khai latency filtering, jitter robust hơn, adaptive payload và stage-specific timeouts.
- Sau khi có backend AU thật: thêm qualification gate, mở model `fixed_city`, rồi mới bật selector AU city-based trong UI.

## Định nghĩa hoàn tất
- Catalog Speed Test chỉ có một nguồn dữ liệu thật.
- Run auto-edge luôn trả rõ `route_fit` và edge đã resolve.
- Throughput được đo theo duration-based strategy thay vì chỉ dựa trên payload MB cố định.
- Việc thêm AU city-based chỉ là thêm target và adapter mới, không phải viết lại engine.

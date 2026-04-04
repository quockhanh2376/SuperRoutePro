export type HelpLanguage = "en" | "vi";

type HelpGuideSection = {
  title: string;
  items: Array<{ name: string; detail: string }>;
};

type HelpGuideContent = {
  modalTitle: string;
  modalSubtitle: string;
  sections: HelpGuideSection[];
};

export const HELP_GUIDE_CONTENT: Record<HelpLanguage, HelpGuideContent> = {
  en: {
    modalTitle: "Help",
    modalSubtitle: "Quick reference for each main button in Super Route Pro.",
    sections: [
      {
        title: "Header & Status",
        items: [
          { name: "Lock / Unlock Repair Mode", detail: "Locked blocks admin-only fixes. Unlock starts an elevated Repair Mode session for this app session, and Lock closes that elevated session again." },
          { name: "Remove Apps", detail: "Open modal to select and remove built-in Windows apps (bloatware)." },
          { name: "Clear Cache", detail: "Open cache cleanup modal to select targets and run cleanup." },
          { name: "Light / Dark", detail: "Switch UI theme between dark mode and light mode." },
          { name: "ONLINE / OFFLINE", detail: "Live internet connectivity status indicator (auto-check every few seconds)." },
          { name: "Latency ms", detail: "Live ping latency monitor to public DNS for quick network health reference." },
        ],
      },
      {
        title: "NIC & Route Actions",
        items: [
          { name: "Network Interfaces table", detail: "Click a NIC row to select interface, auto-fill gateway field, and target route actions." },
          { name: "Active only", detail: "Filter list to only currently active interfaces with valid IPv4." },
          { name: "NIC Refresh", detail: "Reload network interfaces and routing table from system." },
          { name: "ADD", detail: "Add a route with Destination/Subnet/Gateway/Metric to selected interface." },
          { name: "DEL", detail: "Delete route based on Destination + Subnet Mask." },
          { name: "WAN", detail: "Set selected NIC as default internet route (0.0.0.0/0) and clean competing defaults." },
          { name: "FLUSH", detail: "Flush all routes (dangerous). Use when you need full route reset." },
          { name: "Persist on startup", detail: "When enabled, WAN action saves one unified startup replay config for the selected WAN and custom routes after reboot." },
        ],
      },
      {
        title: "Network Fix Tools",
        items: [
          { name: "Flush DNS", detail: "Clear resolver cache (`ipconfig /flushdns`)." },
          { name: "Renew IP", detail: "Release and renew DHCP lease (`ipconfig /release && ipconfig /renew`)." },
          { name: "Wi-Fi Info", detail: "Show current wireless adapter/interface details." },
          { name: "Clear ARP", detail: "Flush ARP cache to resolve stale address mappings." },
          { name: "Reset TCP/IP", detail: "Reset IP stack configuration (`netsh int ip reset`)." },
          { name: "Reset Winsock", detail: "Reset socket catalog (`netsh winsock reset`)." },
          { name: "Reset Firewall", detail: "Reset Windows Firewall to defaults." },
          { name: "Battery Info", detail: "Open battery health summary focused on wear and expected runtime." },
        ],
      },
      {
        title: "Diagnostics, Ping & Output",
        items: [
          { name: "Display DNS Cache", detail: "Print DNS cache entries to command output." },
          { name: "Reset WinHTTP Proxy", detail: "Clear WinHTTP proxy settings to direct mode." },
          { name: "Restart Adapters", detail: "Restart active physical network adapters." },
          { name: "Scan IP", detail: "Scan hosts in active subnet and show reachable devices." },
          { name: "Port Test", detail: "Run Test-NetConnection to verify host/port accessibility." },
          { name: "NSLookup", detail: "Resolve host via selected DNS server and print result." },
          { name: "Ping / fping mode", detail: "Switch between single-target ping and multi-target fping-like monitor." },
          { name: "Start / Stop / Tracert", detail: "Run continuous ping, stop monitor, or trace route to current target." },
          { name: "Output Console chips", detail: "Switch command/routing view, refresh routing snapshot, and clear logs." },
          { name: "Donate", detail: "Open donation QR modal." },
          { name: "Help", detail: "Open this help guide to review all main actions quickly." },
        ],
      },
    ],
  },
  vi: {
    modalTitle: "Trợ giúp",
    modalSubtitle: "Hướng dẫn nhanh các nút chính trong Super Route Pro.",
    sections: [
      {
        title: "Thanh trên cùng & Trạng thái",
        items: [
          { name: "Lock / Unlock Repair Mode", detail: "Khi Locked, app chặn các tác vụ cần quyền admin. Bấm Unlock để mở phiên Repair Mode nâng quyền cho đúng session app hiện tại, và bấm Lock để đóng lại phiên đó." },
          { name: "Remove Apps", detail: "Mở cửa sổ gỡ ứng dụng mặc định của Windows (bloatware), chọn app cần gỡ rồi chạy remove." },
          { name: "Clear Cache", detail: "Mở cửa sổ dọn cache hệ thống/trình duyệt; chọn mục cần dọn và bắt đầu cleanup." },
          { name: "Light / Dark", detail: "Đổi giao diện giữa sáng và tối." },
          { name: "ONLINE / OFFLINE", detail: "Hiển thị trạng thái có Internet theo thời gian thực, tự kiểm tra định kỳ." },
          { name: "Latency ms", detail: "Độ trễ ping hiện tại để bạn theo dõi nhanh chất lượng kết nối mạng." },
        ],
      },
      {
        title: "Quản lý NIC & Route",
        items: [
          { name: "Bảng Network Interfaces", detail: "Bấm vào từng NIC để chọn interface thao tác; app tự điền Gateway tương ứng vào form." },
          { name: "Active only", detail: "Chỉ hiển thị các card mạng đang hoạt động và có IPv4 hợp lệ." },
          { name: "NIC Refresh", detail: "Tải lại danh sách card mạng và bảng định tuyến mới nhất từ hệ thống." },
          { name: "ADD", detail: "Thêm route mới theo Destination/Subnet/Gateway/Metric cho NIC đang chọn." },
          { name: "DEL", detail: "Xóa route theo Destination + Subnet Mask." },
          { name: "WAN", detail: "Đặt NIC đã chọn làm đường ra Internet mặc định (default route 0.0.0.0/0), đồng thời dọn default route cạnh tranh." },
          { name: "FLUSH", detail: "Xóa toàn bộ route hiện có (nguy hiểm), dùng khi cần reset routing từ đầu." },
          { name: "Persist on startup", detail: "Nếu bật, mỗi lần bấm WAN app sẽ lưu một cấu hình startup thống nhất để tự áp WAN và các route custom sau khi khởi động lại máy." },
        ],
      },
      {
        title: "Network Fix Tools",
        items: [
          { name: "Flush DNS", detail: "Xóa cache DNS (`ipconfig /flushdns`) để tránh bản ghi cũ hoặc sai." },
          { name: "Renew IP", detail: "Release + renew DHCP để xin lại IP mới từ modem/router." },
          { name: "Wi-Fi Info", detail: "Xem chi tiết trạng thái Wi-Fi hiện tại (SSID, tốc độ, tín hiệu...)." },
          { name: "Clear ARP", detail: "Xóa ARP cache để cập nhật lại ánh xạ IP-MAC." },
          { name: "Reset TCP/IP", detail: "Reset stack TCP/IP khi gặp lỗi mạng khó đoán nguyên nhân." },
          { name: "Reset Winsock", detail: "Reset Winsock catalog khi lỗi socket/network API." },
          { name: "Reset Firewall", detail: "Đưa Windows Firewall về mặc định." },
          { name: "Battery Info", detail: "Mở bảng pin: độ chai, dung lượng còn lại, chu kỳ sạc và thời gian dùng ước tính." },
        ],
      },
      {
        title: "Diagnostics, Ping & Output",
        items: [
          { name: "Display DNS Cache", detail: "In danh sách cache DNS hiện tại ra khung Command Output." },
          { name: "Reset WinHTTP Proxy", detail: "Xóa cấu hình proxy WinHTTP về direct để khắc phục lỗi kết nối do proxy." },
          { name: "Restart Adapters", detail: "Khởi động lại các card mạng vật lý đang hoạt động." },
          { name: "Scan IP", detail: "Quét subnet đang dùng để tìm host đang online trong mạng LAN." },
          { name: "Port Test", detail: "Kiểm tra truy cập host/port (mở hay chặn) bằng Test-NetConnection." },
          { name: "NSLookup", detail: "Phân giải tên miền theo DNS chỉ định và xem kết quả trả về." },
          { name: "Ping / fping mode", detail: "Chọn chế độ ping một đích hoặc fping-like nhiều đích cùng lúc." },
          { name: "Start / Stop / Tracert", detail: "Bắt đầu theo dõi ping liên tục, dừng theo dõi, hoặc chạy tracert tới mục tiêu hiện tại." },
          { name: "Output Console chips", detail: "Chuyển tab Command/Routing, refresh snapshot routing, và xóa log nhanh." },
          { name: "Donate", detail: "Mở cửa sổ QR để ủng hộ tác giả." },
          { name: "Help", detail: "Mở bảng hướng dẫn này để xem mô tả chức năng từng nút." },
        ],
      },
    ],
  },
};

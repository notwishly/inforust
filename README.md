# Wishly — Rust + WebAssembly

Đây vẫn là **website tĩnh**, không phải app. HTML/CSS giữ giao diện và hiệu ứng trình duyệt; Rust/WASM xử lý profile chất lượng, trạng thái vật lý của tuyết, tích tụ/tan tuyết, giọt nước và tính toán đường biên tuyết. JavaScript chỉ nối DOM/pointer event với engine và vẽ các điểm đã được Rust tính sẵn.

## Vì sao không chuyển HTML/CSS sang Rust?

GitHub Pages không chạy Rust server. Rust phải biên dịch thành `.wasm`; trình duyệt vẫn cần DOM/CSS/JS cho giao diện, CSS animation, WebSocket Discord, audio và API web. Chuyển DOM qua WASM thường làm tăng độ phức tạp mà không tăng tốc.

## Chạy local

File HTML gốc tham chiếu `img/bg.jpg`. Bản upload trong cuộc trò chuyện chỉ có `index.html`, vì vậy nếu repo gốc có thư mục `img/` hoặc các tài nguyên ảnh/âm thanh khác, hãy giữ/copy chúng vào cùng vị trí khi thay file.

Yêu cầu Rust, `wasm-pack`, Python 3:

```bash
wasm-pack build --target web --release --out-dir pkg
python3 -m http.server 8080
```

Mở `http://localhost:8080`.

## Deploy GitHub Pages

1. Đưa toàn bộ nội dung repo lên nhánh `main`.
2. Vào **Settings → Pages → Build and deployment → Source: GitHub Actions**.
3. Workflow `.github/workflows/deploy.yml` chạy `cargo test --lib`, build Rust thành WASM rồi deploy. Thư mục build Rust được đặt ngoài thư mục deploy để tránh upload file build thừa.
4. Không mở `index.html` bằng `file://`; WASM ES modules cần HTTP(S).

## Profile kiểm thử

- `/?profile=2core`: profile yếu (2 core, 2 GB) — 30 FPS cap, DPR 1, mật độ tuyết 0.6, blur 0.45, tắt lens và glow đắt.
- `/?profile=4core`: profile cân bằng — 45 FPS cap, DPR 1.25, mật độ tuyết 0.65, blur 0.7, tắt lens.
- `/?profile=full`: profile chất lượng cao — 60 FPS cap, DPR tối đa 2, giữ đầy đủ hiệu ứng.
- `?lite=0|1|2` vẫn được hỗ trợ để tương thích URL cũ.
- `prefers-reduced-motion`: tuyết tĩnh, giới hạn vòng cập nhật xuống 1 FPS; các chuyển động CSS được giảm theo thiết lập hệ điều hành.
- `/benchmark.html`: benchmark phép tính WASM tổng quát, không thay thế đo FPS thật của toàn trang.

**Lưu ý:** profile 2-core là mô phỏng chính sách chất lượng, không biến CPU 8-core thành CPU 2-core thật. Đo FPS/độ trễ thực cần Chrome DevTools CPU throttling hoặc thiết bị 2-core thực tế.

## Trạng thái port

Bản này giữ nội dung và bố cục `index.html` gốc. Đã chuyển mô phỏng tuyết sang Rust: tạo/tăng trưởng/tan cụm tuyết, timer, giọt nước và tính toán 21 điểm đường biên cho mỗi cụm. Mỗi frame gửi descriptor của mọi bề mặt qua một lần `step_all` với scratch buffer được tái sử dụng; cập nhật giọt nước cũng gộp vào một lần gọi và chỉ tạo mảng khi có giọt. JavaScript vẫn đo DOM, nhận pointer event và vẽ Canvas. Discord/Lanyard, player/audio, nội dung động và DOM/CSS vẫn ở JavaScript/trình duyệt vì chúng phụ thuộc API web, không phải tải tính toán phù hợp để chuyển sang Rust.

Profile Rust điều khiển FPS, DPR, mật độ tuyết, blur và các hiệu ứng đắt. Nếu bộ đo FPS phát hiện thiết bị hụt khung hình liên tục, adapter hạ thêm xuống profile tiết kiệm, giảm DPR/mật độ tuyết và tắt pointer glow. CSS `lite`/`ultra` cũ được giữ để tránh thay đổi giao diện ngoài ý muốn; profile WASM cũng đồng bộ với các class này. Khi WASM không tải được, các phần còn lại của website vẫn hoạt động nhưng lớp tuyết Rust không khởi tạo. Trước khi coi là bản phát hành hoàn tất, cần để GitHub Actions chạy `cargo test` + build WASM và kiểm tra hình ảnh/FPS thực tế trên trình duyệt hoặc thiết bị 2-core. Môi trường phát triển hiện tại không có Rust toolchain nên chưa thể xác nhận biên dịch Rust tại chỗ.

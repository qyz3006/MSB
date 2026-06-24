# Local Build Setup

這份文件以 [`.github/actions/setup/action.yml`](../.github/actions/setup/action.yml) 與 [`.github/workflows/build.yml`](../.github/workflows/build.yml) 為準，整理出在 Windows 本機重現 CI build 環境的做法。

## 適用範圍

- 作業系統：Windows
- Rust target：`x86_64-pc-windows-msvc`
- UI build：Dioxus desktop
- 原生依賴：LLVM / libclang、Node.js、vcpkg 安裝的 OpenCV 4 static

## CI 目前實際使用的版本與設定

GitHub Actions 目前做的事情如下：

- 使用 Rust nightly toolchain
- 安裝 LLVM 21
- 安裝 Node.js，並在 `ui` 目錄執行 `npm install`
- 安裝 Dioxus CLI `0.7.2`
- 用 vcpkg 安裝 `opencv4[contrib,nonfree]:x64-windows-static`
- 設定這些重要環境變數：

```powershell
OPENCV_MSVC_CRT=static
OPENCV_DISABLE_PROBES=environment,pkg_config,cmake,vcpkg_cmake
VCPKGRS_TRIPLET=x64-windows-static
VCPKG_ROOT=D:\repos\vcpkg\vcpkg.exe
VCPKG_INSTALLED_DIR=D:\repos\vcpkg\vcpkg.exe\installed
```

注意：CI 的 cache key 使用 `VCPKG_VERSION=2026.02.27`，但實際 checkout 的 vcpkg 版本是 `2025.10.17`。本機若要對齊 CI，應以 `2025.10.17` 為準。

## 1. 安裝 Rust toolchain

專案根目錄的 [rust-toolchain.toml](../rust-toolchain.toml) 已指定：

- `nightly`
- `x86_64-pc-windows-msvc`
- `clippy`
- `rustfmt`

如果你的環境還沒準備好，可以先執行：

```powershell
rustup toolchain install nightly --profile minimal --component clippy --component rustfmt
rustup target add x86_64-pc-windows-msvc --toolchain nightly
```

## 2. 安裝 LLVM 21

CI 會安裝 LLVM 21，而且專案目前在 [`.cargo/config.toml`](../.cargo/config.toml) 內預設：

```toml
LIBCLANG_PATH = "C:/Program Files/LLVM/bin"
```

所以最簡單的做法是：

- 直接把 LLVM 21 安裝到預設路徑 `C:\Program Files\LLVM`

如果你安裝在別的路徑，請二選一：

- 修改本機 shell 的 `LIBCLANG_PATH`
- 或調整 [`.cargo/config.toml`](../.cargo/config.toml) 裡的 `LIBCLANG_PATH`

可用以下指令確認 `libclang` 存在：

```powershell
Test-Path 'C:\Program Files\LLVM\bin\libclang.dll'
```

## 3. 安裝 Node.js 並下載前端套件

`ui/build.rs` 會在 build 時直接呼叫 `npx @tailwindcss/cli`，所以本機一定要有 Node.js 與 npm。

建議先安裝最新版 Node.js LTS，然後在專案根目錄執行：

```powershell
Set-Location d:\repos\komari-2
Set-Location ui
npm install
Set-Location ..
```

## 4. 安裝 Dioxus CLI

CI 用的是 `dioxus-cli@0.7.2`。本機建議也用同版：

```powershell
cargo install cargo-binstall
cargo binstall dioxus-cli@0.7.2 --no-confirm
```

如果你不想額外安裝 `cargo-binstall`，也可以改用：

```powershell
cargo install dioxus-cli --version 0.7.2 --locked
```

安裝完成後確認：

```powershell
dx --version
```

## 5. 準備 vcpkg 與 OpenCV

### 5.1 下載並切到和 CI 相同的 vcpkg 版本

```powershell
Set-Location d:\repos\komari-2
git clone https://github.com/Microsoft/vcpkg.git .\vcpkg
Set-Location .\vcpkg
git checkout 2025.10.17
.\bootstrap-vcpkg.bat -disableMetrics
```

### 5.2 對齊 CI 的 triplet 設定

CI 會把以下內容附加到兩個 triplet 檔案：

```cmake
set(VCPKG_BUILD_TYPE release)
```

請手動加入到：

- `vcpkg/triplets/x64-windows.cmake`
- `vcpkg/triplets/x64-windows-static.cmake`

這樣能和 CI 一樣只建 release 版本套件，避免不必要的 debug 套件編譯。

### 5.3 安裝 OpenCV

```powershell
Set-Location d:\repos\komari-2\vcpkg
.\vcpkg.exe install --clean-after-build --recurse opencv4[contrib,nonfree]:x64-windows-static
```

第一次安裝會很久，這是正常的。

## 6. 設定本機環境變數

專案內的 [`.cargo/config.toml`](../.cargo/config.toml) 已經提供了以下設定：

```toml
OPENCV_MSVC_CRT = "static"
OPENCV_DISABLE_PROBES = "environment,pkg_config,cmake,vcpkg_cmake"
VCPKGRS_TRIPLET = "x64-windows-static"
LIBCLANG_PATH = "C:/Program Files/LLVM/bin"
```

你至少還需要補上 `VCPKG_ROOT`。若想完全對齊 CI，連 `VCPKG_INSTALLED_DIR` 也一起設：

```powershell
$env:VCPKG_ROOT = 'd:\repos\komari-2\vcpkg'
$env:VCPKG_INSTALLED_DIR = 'd:\repos\komari-2\vcpkg\installed'
```

如果你想永久生效，可以把它們設成使用者環境變數，或加入 `%USERPROFILE%\.cargo\config.toml` 的 `[env]` 區段。

## 7. 開始 build

在專案根目錄執行：

```powershell
Set-Location d:\repos\komari-2
dx build --package ui
```

release build：

```powershell
Set-Location d:\repos\komari-2
dx build --release --package ui
```

產物位置和 CI 相同：

- debug: `target/dx/ui/debug/windows/app`
- release: `target/dx/ui/release/windows/app`

## 8. 本機驗證命令

如果你想確認本機環境和 CI 一致，可以依序執行：

```powershell
Set-Location d:\repos\komari-2
cargo fmt --check
cargo clippy -- -D warnings
cargo test -- --no-capture
dx build --release --package ui
```

其中 `dx build --release --package ui` 就是 CI 真正的 release build 命令。

## 9. 哪些東西不是本機必裝

### protoc

CI 有額外安裝 `protoc`，但這個專案的 [backend/build.rs](../backend/build.rs) 會透過 `protoc-bin-vendored` 自行提供 `PROTOC`：

```rust
let protoc_path = protoc_bin_vendored::protoc_bin_path().unwrap();
std::env::set_var("PROTOC", protoc_path);
```

所以在一般本機 build 情境下，`protoc` 不是必要前置條件。

### sccache

CI 會用 `sccache` 加速編譯，但本機不裝也不影響正確 build。

## 10. 常見問題

### 找不到 libclang

優先檢查：

- LLVM 是否安裝到 `C:\Program Files\LLVM`
- `LIBCLANG_PATH` 是否指向 LLVM 的 `bin`

### OpenCV 偵測失敗

優先檢查：

- `VCPKG_ROOT` 是否正確
- `VCPKGRS_TRIPLET` 是否為 `x64-windows-static`
- 是否真的安裝了 `opencv4[contrib,nonfree]:x64-windows-static`

### `npx` 或 Tailwind build 失敗

優先檢查：

- 是否已安裝 Node.js
- 是否已在 `ui` 目錄執行 `npm install`

### 程式啟動時缺少元件

除了 build 依賴外，執行桌面版程式還建議安裝：

- Visual C++ Redistributable 2015-2022
- Microsoft WebView2 Runtime

如果要啟用 GPU，依 [docs/troubleshooting.md](./troubleshooting.md) 的說明，還需要 CUDA 12.x。
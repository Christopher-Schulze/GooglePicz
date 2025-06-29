<div align="center">
  <img src="logo/image.png" alt="GooglePicz Logo" width="200" style="border-radius: 20px; box-shadow: 0 4px 8px rgba(0,0,0,0.1);">
  
  # 🖼️ GooglePicz

  [![Rust](https://img.shields.io/badge/Rust-1.70+-orange?logo=rust)](https://www.rust-lang.org/)
  [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
  [![Project Status: WIP](https://img.shields.io/badge/status-WIP-yellow)](https://github.com/Christopher-Schulze/GooglePicz)
</div>


> A work-in-progress native Google Photos client for macOS and Windows, built with Rust for maximum performance and efficiency.

## 🚧 Project Status: Early Development

This project is currently in active development and not yet ready for production use. We're building a native desktop solution to fill the gap left by Google's lack of official desktop clients.

## 🎯 Project Goals

- 🚀 Native performance with Rust
- 🔒 Secure OAuth2 authentication
- ⚡ GPU-accelerated image rendering
- 📂 Local cache for offline access
- 🎨 Cross-platform UI with Iced

## 🛠️ Technical Stack

- **Language**: Rust 1.70+
- **UI Framework**: Iced (wgpu backend)
- **Storage**: SQLite
- **Authentication**: OAuth2
- **Target Platforms**: macOS & Windows

## 📦 Getting the Code

```bash
git clone https://github.com/Christopher-Schulze/GooglePicz.git
cd GooglePicz
```

## 🏗️ Project Structure

```
GooglePicz/
├── app/          # Main application
├── auth/         # OAuth2 authentication
├── api_client/   # Google Photos API client
├── ui/           # User interface (Iced)
├── cache/        # Local SQLite cache
└── sync/         # Background synchronization
```

## 📝 Documentation

See [docs/DOCUMENTATION.md](docs/DOCUMENTATION.md) for detailed technical documentation.

## 📄 License

MIT - See [LICENSE](LICENSE) for details.

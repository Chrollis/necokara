# Acknowledgments

Necokara uses DeepSeek as an AI-assisted development tool for code generation, documentation, translation, and problem solving. All AI-generated content is reviewed and maintained by the project maintainer.

## Dependencies

### Runtime

| Dependency                 | Purpose                          | License                            |
| -------------------------- | -------------------------------- | ---------------------------------- |
| Python 3.12 embeddable     | Python runtime                   | Python Software Foundation License |
| FFmpeg / FFprobe           | Media decoding and processing    | GPL（BtbN FFmpeg-Builds 预编译）   |
| faster-whisper             | Speech recognition               | MIT                                |
| stable-ts / stable-whisper | Lyric alignment                  | MIT                                |
| demucs-infer               | Vocal separation                 | MIT                                |
| librosa                    | Audio analysis                   | ISC                                |
| janome                     | Japanese tokenization            | Apache-2.0                         |
| numpy                      | Numerical computation            | BSD-3-Clause                       |
| torch                      | Deep learning backend for Demucs | BSD-style / multiple               |

### Development / Build

| Dependency    | Purpose                   | License          |
| ------------- | ------------------------- | ---------------- |
| Node.js / npm | JavaScript toolchain      | MIT              |
| Rust / Cargo  | Rust toolchain            | MIT / Apache-2.0 |
| React         | UI library                | MIT              |
| Vite          | Frontend build tool       | MIT              |
| TypeScript    | Type checking             | Apache-2.0       |
| Tauri 2       | Desktop application shell | MIT / Apache-2.0 |

## Third-Party Licenses

- FFmpeg: [COPYING.GPLv3](https://github.com/FFmpeg/FFmpeg/blob/master/COPYING.GPLv3)
  - Pre-built source：[BtbN/FFmpeg-Builds](https://github.com/BtbN/FFmpeg-Builds) (`ffmpeg-master-latest-win64-gpl`)
- Python runtime: [license.html](https://docs.python.org/3/license.html)
  - Download source: [python.org](https://www.python.org/ftp/python) (`python-3.12.10-embed-amd64`)

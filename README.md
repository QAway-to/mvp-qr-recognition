# mvp-qr-recognition

> QR code recognition compiled to WebAssembly — handles blur, low-light, perspective distortion, and payment formats in the browser.

Pure Rust image-processing pipeline compiled to WASM via `wasm-bindgen`. Ships as a Next.js app that runs recognition client-side with no server round-trips. Decodes payment QR formats including EMV, SBP (Russia Fast Payments), and ST.00012.

## Features

- **Difficult conditions** — blur, partial occlusion, glare, uneven lighting, perspective skew
- **Multi-QR detection** — finds and decodes several codes in a single frame
- **Payment QR parsing** — extracts structured fields from EMV, SBP, and ST.00012 payloads
- **Relevance ranking** — selects the most payment-relevant code when multiple are present
- **Client-side** — WASM module runs entirely in the browser, zero latency from server calls
- **Cross-platform** — Web, Mobile Web, and embeddable in native apps via the compiled artifact

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Core library | Rust (`rxing`, `rqrr`) |
| WASM bindings | `wasm-bindgen`, `wasm-pack` |
| Web interface | Next.js 14, React |
| Image preprocessing | Custom Rust pipeline |

## Project Structure

```
├── crates/
│   ├── qr-core/
│   │   ├── preprocessing/   # Image normalization, thresholding
│   │   ├── detection/       # Finder pattern location
│   │   ├── decoding/        # rxing + rqrr dual-decoder
│   │   └── payment/         # EMV / SBP / ST.00012 parsers
│   └── qr-wasm/             # wasm-bindgen bindings
├── pages/                   # Next.js app
├── scripts/                 # Dataset generation helpers
└── tests/                   # Test images (blur, occlusion, etc.)
```

## Getting Started

**Prerequisites:** Rust toolchain, `wasm-pack`

```bash
# Build the WASM module
cd crates/qr-wasm && wasm-pack build --target web

# Run the web demo
npm install && npm run dev
```

## License

MIT

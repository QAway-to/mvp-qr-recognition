/** @type {import('next').NextConfig} */

const path = require("path");

/** @type {import('next').NextConfig} */
const nextConfig = {
    reactStrictMode: true,
    // Enable WASM support
    webpack: (config, { isServer }) => {
        config.experiments = {
            ...config.experiments,
            asyncWebAssembly: true,
            layers: true,
        };

        // Fix for "import.meta" error in onnxruntime-web by using the CommonJS build
        config.resolve.alias = {
            ...config.resolve.alias,
            "onnxruntime-web": path.join(__dirname, "node_modules/onnxruntime-web/dist/ort.all.min.js"),
        };

        // Manual copy of ONNX Runtime WASM files handled via git to avoid build errors (webpack/terser issues)

        return config;
    },
    // Headers for SharedArrayBuffer (required for some WASM features)
    async headers() {
        return [
            {
                source: '/(.*)',
                headers: [
                    {
                        key: 'Cross-Origin-Opener-Policy',
                        value: 'same-origin',
                    },
                    {
                        key: 'Cross-Origin-Embedder-Policy',
                        value: 'require-corp',
                    },
                ],
            },
        ];
    },
};

module.exports = nextConfig;

/** @type {import('next').NextConfig} */
const CopyPlugin = require("copy-webpack-plugin");
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

        // Fix for "import.meta" error in onnxruntime-web
        config.resolve.alias = {
            ...config.resolve.alias,
            "onnxruntime-web/all": path.join(__dirname, "node_modules/onnxruntime-web/dist/ort.all.min.mjs"),
        };

        // Copy ONNX Runtime WASM files to public directory
        if (!isServer) {
            config.plugins.push(
                new CopyPlugin({
                    patterns: [
                        {
                            from: path.join(__dirname, "node_modules/onnxruntime-web/dist/*.wasm"),
                            to: path.join(__dirname, "public/pkg/[name][ext]"),
                        },
                    ],
                })
            );
        }

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

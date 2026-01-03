/** @type {import('next').NextConfig} */
const nextConfig = {
    reactStrictMode: true,
    // Enable WASM support
    webpack: (config) => {
        config.experiments = {
            ...config.experiments,
            asyncWebAssembly: true,
        };
        return config;
    },
};

module.exports = nextConfig;

# ── Stage 1: Build WASM + Next.js ────────────────────────────────────────────
FROM rust:1.82-slim AS builder

# System deps
RUN apt-get update && apt-get install -y \
    curl \
    nodejs \
    npm \
    && rm -rf /var/lib/apt/lists/*

# Node 20 (apt gives old version)
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*

# wasm-pack
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

WORKDIR /app

# Copy Rust workspace first for layer caching
COPY Cargo.toml Cargo.lock* ./
COPY crates/ ./crates/

# Build WASM (output → public/pkg/)
RUN wasm-pack build crates/qr-wasm --target web --out-dir /app/public/pkg --release

# Copy the rest of the app
COPY package.json package-lock.json* ./
COPY pages/ ./pages/
COPY styles/ ./styles/
COPY lib/ ./lib/
COPY public/ ./public/
COPY scripts/ ./scripts/
COPY next.config.js ./

# Install JS deps and build Next.js
RUN npm ci && npm run build


# ── Stage 2: Runtime ─────────────────────────────────────────────────────────
FROM node:20-alpine AS runner

ENV NODE_ENV=production
ENV PORT=3000

WORKDIR /app

# Only what Next.js needs to run
COPY --from=builder /app/package.json ./
COPY --from=builder /app/node_modules ./node_modules
COPY --from=builder /app/.next ./.next
COPY --from=builder /app/public ./public

EXPOSE 3000

CMD ["node_modules/.bin/next", "start", "-p", "3000"]

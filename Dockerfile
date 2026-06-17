FROM node:20-alpine

ENV NODE_ENV=production
ENV PORT=3000

WORKDIR /app

# Copy manifest + postinstall script so `npm ci` can run its postinstall hook
COPY package.json package-lock.json* ./
COPY scripts/ ./scripts/
RUN npm ci

# Copy the rest and build
COPY . .
RUN npm run build

EXPOSE 3000

CMD ["node_modules/.bin/next", "start", "-p", "3000"]

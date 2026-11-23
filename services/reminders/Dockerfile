ARG NODE_IMAGE=node:24-bookworm-slim
FROM ${NODE_IMAGE}
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci --omit=dev --ignore-scripts --no-audit --no-fund && npm cache clean --force
COPY src ./src
RUN mkdir /data && chown node:node /data
USER node
ENV NODE_ENV=production TODO_PUSH_HOST=0.0.0.0 TODO_PUSH_PORT=8787 TODO_PUSH_DATA_FILE=/data/reminders.json
EXPOSE 8787
VOLUME ["/data"]
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s CMD node -e "fetch('http://127.0.0.1:8787/health').then(r=>{if(!r.ok)process.exit(1)}).catch(()=>process.exit(1))"
CMD ["node", "src/main.mjs"]

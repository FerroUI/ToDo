# Optional reminder service

This is the deployable Web Push scheduler for FerroUI To Do. It runs separately
from the static site and is disabled until a browser explicitly enrolls.

See [BACKGROUND-PUSH.md](../../docs/BACKGROUND-PUSH.md) for deployment, enrollment,
protocol guarantees, privacy, limits, recovery, and acceptance steps.

```sh
npm ci --ignore-scripts
npm test
npm run keys
cp .env.example .env
# Fill .env with the generated keys, the app's exact origin, and your contact.
node --env-file=.env src/main.mjs
```

The durable service requires Linux and a local filesystem that supports atomic
rename and file/directory fsync. Use the supplied container on other hosts. Keep
one writer per data directory, terminate TLS at a reverse proxy, and protect the
private environment and data files. No deployment is performed by the app.

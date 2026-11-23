# Optional browser background reminders

The browser app can schedule reminders with a separately deployed Web Push
service. A browser explicitly enrolls in Settings with its service URL, public
VAPID key, and an administrator-provided enrollment key. The service can then
wake that app's service worker when its page is closed. The GitHub Pages/static
website by itself cannot send those pushes.

This repository supplies the server and client implementation. It does not
provision a server, subscribe the user's browser, or claim an externally tested
OS notification delivery. The desktop app has its own native background
reminder setting; these Web Push controls appear only in the browser host.

## Deploy the service

Use a maintained Node.js 22 or newer release on Linux. The supplied container
uses the Node.js 24 Debian Bookworm slim base. Linux is required for the durable store's file and directory
fsync contract; Docker provides that environment on other hosts. Use local
persistent storage, one service process per store, and a TLS reverse proxy.
Do not mount the data directory on a filesystem without equivalent rename and
fsync guarantees.

```sh
cd services/reminders
npm ci --ignore-scripts
npm test
npm run keys
cp .env.example .env
```

Put the three generated values in `.env`: `TODO_PUSH_VAPID_PUBLIC_KEY`,
`TODO_PUSH_VAPID_PRIVATE_KEY`, and `TODO_PUSH_ENROLLMENT_KEY`. Keep the private
VAPID key and the enrollment key secret. The public VAPID key is intended to be
shared with browsers. Set `TODO_PUSH_VAPID_SUBJECT` to an actual administrator
contact (`mailto:` or a public HTTPS URL) and set `TODO_PUSH_ORIGINS` to the exact
application origins allowed to enroll, for example `https://ferroui.com`.
Origins have no path, trailing slash, or wildcard. Multiple origins are comma
separated. Rotate the enrollment key by changing the environment and restarting;
already enrolled browsers use their own independent capabilities.

Start with the environment file:

```sh
node --env-file=.env src/main.mjs
```

The default listener is `127.0.0.1:8787`. A reverse proxy can expose this at a
separate HTTPS origin such as `https://reminders.example.org/`, forwarding the
`/health` and `/v1/…` paths unchanged. Preserve the browser's `Origin` header and
allow `OPTIONS`, `GET`, `POST`, `PUT`, and `DELETE`. The app uses authenticated
CORS requests without cookies. The service does not trust `X-Forwarded-For`;
rate limits therefore apply to the proxy address when a proxy is present. Apply
an additional client-IP limit at the proxy if needed. Restrict request bodies to
256 KiB and do not log authorization or enrollment headers or request bodies.

Alternatively, build the supplied image and bind its port only to loopback for
the TLS proxy. Use a private persistent volume mounted at `/data`, supply the
same environment, and run a single replica. The image runs as the `node` user.
The container configuration must retain `TODO_PUSH_HOST=0.0.0.0`; the native
`.env.example` loopback setting is for running Node directly. Pin and update the
base image through your deployment process when applying security updates. For
reproducible production builds, set the Docker build argument `NODE_IMAGE` to
the base image's registry-verified digest; the default major-version tag follows
maintained patch updates and is not an immutable digest.

`GET /health` reports process/storage health. An ambiguous storage failure makes
it return 503 and stops further state mutations and delivery snapshots.
`GET /v1/config` requires an allowed browser origin and returns only the public
VAPID key and service limits. There is no unauthenticated task listing.

### Configuration

| Variable | Default | Meaning |
| --- | --- | --- |
| `TODO_PUSH_ORIGINS` | required | Exact permitted browser application origins |
| `TODO_PUSH_ENROLLMENT_KEY` | required | Random base64url enrollment secret, at least 32 bytes |
| `TODO_PUSH_VAPID_PUBLIC_KEY` | required | Public P-256 application server key |
| `TODO_PUSH_VAPID_PRIVATE_KEY` | required | Private VAPID key, server only |
| `TODO_PUSH_VAPID_SUBJECT` | required | Real administrator contact |
| `TODO_PUSH_HOST`, `TODO_PUSH_PORT` | `127.0.0.1`, `8787` | Listen address |
| `TODO_PUSH_DATA_FILE` | `data/reminders.json` | Private durable store |
| `TODO_PUSH_MAX_INSTALLATIONS` | `1000` | Maximum browser installations; accepted range 1–10000 |
| `TODO_PUSH_MAX_JOBS` | `200` | Maximum pending jobs per installation; accepted range 1–1000 |
| `TODO_PUSH_HORIZON_DAYS` | `366` | How far ahead schedules may be uploaded; accepted range 1–730 |
| `TODO_PUSH_LEASE_DAYS` | `366` | Maximum installation lease renewed on schedule updates; accepted range 1–730 |
| `TODO_PUSH_EXPIRY_MINUTES` | `60` | Maximum delay after a reminder time before it is discarded; accepted range 1–1440 |
| `TODO_PUSH_EXTRA_ENDPOINT_HOSTS` | empty | Additional exact push-provider DNS names |
| `TODO_PUSH_ALLOW_LOCAL_HTTP` | `0` | Development-only opt-in for exact localhost HTTP application origins |

The default push-provider hosts are `fcm.googleapis.com`,
`updates.push.services.mozilla.com`, and `web.push.apple.com`. A deployment that
needs another browser's provider can configure its exact hostname. Do not add
arbitrary user-supplied hosts. Every outbound connection also validates DNS
answers and rejects private, loopback, mapped, link-local, documentation,
multicast and reserved addresses. Redirects, URL credentials, nonstandard ports,
HTTP endpoints and proxy tunneling are not accepted.

## Enroll a browser

1. Open the normal installed app over HTTPS. On iPhone, use an installed Home
   Screen web app with notification support. The demo and `nooffline` preview
   deliberately cannot enroll or reuse a personal subscription.
2. In Settings, open **Browser background reminders**. Enter the service's HTTPS
   URL, its public VAPID key, and its enrollment key. The client verifies the
   fetched service configuration against that public key.
3. Choose **Enable background reminders** and grant browser notification
   permission. The enrollment field is cleared after use. The client creates a
   push subscription with `userVisibleOnly: true` and the selected application
   server key.
4. Check the scheduled count and renewal date in Settings. Keep the page open
   until the schedule update completes before relying on closed-page delivery.

**Refresh background reminders** verifies/replaces a missing or expired browser
subscription and uploads the current active schedule. Browser subscription
changes also attempt renewal from the worker. If permission or browser policy
prevents that, Settings asks for an explicit refresh. Once an expired service
installation has been purged, disable and enroll again with the administrator's
enrollment key.

**Disable background reminders** first turns off the worker's local delivery
gate, clears its plan, and unsubscribes. It then deletes the remote installation.
If the network is unavailable, only the credentials needed for pending cleanup
remain in the private sidecar; cleanup retries when the app reconnects or opens.
**Clean up background subscription** repeats cleanup after a failed enrollment
or disable. Permission revocation uses the same local cancellation behavior.

## Scheduling, identity and cancellation

Only the active Microsoft-account workspace or the active local workspace is
scheduled. Completion, deletion, reminder edits, task renames, imports and
account changes rebuild the local plan and replace the remote schedule. No
Microsoft account identifier is sent to the scheduler: each workspace is
represented by a hash salted with the installation's random seed. A local
import gets a new local generation so imported entity IDs cannot inherit the
previous document's delivery receipts. Switching back to an account retains
that account's occurrence identities.

The client uploads the earliest eligible reminders up to the configured count
and time horizon. Settings reports omitted reminders. It includes reminders
that have just become overdue while they remain inside the expiry window.
Already completed or notified tasks are excluded. The server lease is also
bounded by a browser subscription's known expiration time. Reminders beyond
that renewal date require the installation to be refreshed before delivery.

Task reminders are civil dates and local clock minutes. Upload converts them to
UTC instants in the current device timezone. A spring-forward gap fires at the
first available minute after the gap, matching the app's civil-clock due check;
a repeated clock minute uses the first occurrence. A wholly skipped local date
is omitted. Reopening the app after a timezone change rebuilds the schedule. A
closed app cannot discover that the device changed timezones, so the server
continues using the last uploaded instants until the app runs again.

Each schedule update uses an authenticated `If-Match` revision. A stale client
must read the latest revision and rebuild from the current durable workspace;
it cannot blindly replay an earlier account's request. All jobs are replaced
together. An occurrence identity is independent of title, so renaming a task
updates the pending text without generating a second notification. The worker
displays the title in its current local plan, not an older message payload.

The worker checks installation, workspace, job ID, delivery time and expiry
against its own private plan before showing anything. That gate can reject a
canceled queued push even when the server has not yet received an offline edit.
However, a service cannot learn about changes made only on another device while
this browser is closed, and a tab closed before its local/remote update finishes
may still have an older schedule. A notification already accepted for display
cannot be retracted reliably. Delivery depends on OS/browser policy, network
availability and the configured service; it is not an exact-time alarm.

Server queue claims are persisted before sending. Transient network errors,
408, 429 and 5xx responses get bounded exponential retries (at most eight
attempts and never beyond expiry). `Retry-After` is honored within a 15-minute
retry-delay cap. A 404/410 subscription response removes that endpoint and its
jobs. The push TTL is the remaining expiry window, not the library's default.
Each outbound attempt also has an absolute ten-second deadline, including DNS,
bounded by remaining TTL; provider response bodies are not read or retained.
Expired installation leases never send jobs.

Push transport can redeliver after lost acknowledgements. The worker and open
page share atomic IndexedDB occurrence receipts, so a background delivery is
not repeated when the app later ticks the same reminder. A failed display
releases its claim; successful occurrences retain their receipt while the
reminder remains in the document. Changed/deleted occurrences get a seven-day
retirement grace for undo. Notification tags provide an additional OS-level
replacement key. Foreground reminders remain available after a service lease
expires, and optional push-storage failures use the ordinary foreground
notification path after checking that the task and workspace are still current.

## Private storage and service boundaries

The page and worker use a scope-specific IndexedDB database named
`ferroui.todo.push.v1:<app-scope>`. It holds the installation capability,
subscription configuration, local schedule and delivery receipts. None of this
is part of task JSON, undo/redo history, Graph synchronization, OneDrive app data,
portable archives or PST exports. The temporary enrollment key is never stored.

The scheduler receives the browser's endpoint/encryption keys, random
installation ID, salted workspace identifier, reminder IDs, titles and times.
It does not receive Microsoft access/refresh tokens, account email addresses,
task notes, files or the complete task document. Protect the service data file:
its schedule contains task titles, and its push subscription material remains
private even though installation bearer capabilities are stored only as hashes.
Its environment file additionally contains the VAPID and enrollment secrets.

The API requires an allowed Origin, limits request/header sizes, rejects
unexpected body fields, gates enrollment with the administrator key, limits
requests and installations, and authenticates every installation operation with
its own 32-byte random bearer capability. The administrator enrollment key is
not sufficient to enumerate or read already enrolled installations. Cryptography
and VAPID/encryption are provided by the pinned `web-push` dependency, not a
custom cryptographic implementation.

## Durability and recovery

The server serializes changes, writes a private temporary file, fsyncs it,
atomically renames it, and fsyncs the containing directory before acknowledging
the request. Its store is limited to 64 MiB and validated before startup. A lock
file enforces one writer. Graceful SIGTERM/SIGINT waits for active delivery and
state operations, then releases the lock. Back up the private store and server
keys together under the deployment's access controls.

After a crash, confirm that the old process is no longer running before removing
the `reminders.json.lock` file and restarting. Never remove a live process's
lock. A corrupt store must be restored from a valid backup; it is not silently
replaced with an empty file. If rename succeeds but the final directory fsync
fails, the service stops further mutations/delivery and reports unhealthy state
until the filesystem is checked and the process is restarted. This prevents a
stale in-memory snapshot from overwriting an ambiguously committed schedule.

## Validation and external acceptance

Run `npm ci --ignore-scripts && npm test` in `services/reminders` for the API,
queue, endpoint/DNS, durable-store and injected-failure tests. Run
`node --test scripts/push-*.test.mjs scripts/notifications.test.mjs` at the repo
root for schedule extraction, account isolation, subscription lifecycle,
storage/foreground fallback, receipt and service-worker tests. The native
Settings browser scenario exercises disabled, unavailable and enabled layouts,
draft retention, password-field clearing and narrow-width accessibility without
enrolling a real subscription.

Deployment acceptance still requires a real HTTPS service and a real browser:
enroll, schedule a near-future task, close every app page, and observe an OS
notification within the configured expiry window. Then separately verify
completion/edit cancellation, reopening without duplication, account switching,
subscription refresh, blocked permission, service/network outage, and disable.
Browser policies differ, so repeat this acceptance on each supported platform.
These steps intentionally remain outside automated tests and no live service
deployment or browser subscription is performed by the repository workflow.

## Primary references

- [W3C Push API](https://www.w3.org/TR/push-api/): subscription, worker events,
  renewal and delivery model.
- [MDN Push API](https://developer.mozilla.org/en-US/docs/Web/API/Push_API) and
  [PushManager.subscribe](https://developer.mozilla.org/en-US/docs/Web/API/PushManager/subscribe):
  browser support, explicit permission, application server keys and visible pushes.
- [MDN pushsubscriptionchange](https://developer.mozilla.org/en-US/docs/Web/API/ServiceWorkerGlobalScope/pushsubscriptionchange_event):
  subscription changes and the limits of background renewal.
- [web-push](https://github.com/web-push-libs/web-push): VAPID, encryption,
  push request options and TTL. The exact dependency is locked in
  `services/reminders/package-lock.json`.
- [Node.js release policy](https://nodejs.org/en/about/previous-releases): use a
  maintained LTS runtime for deployment.

# Native cache-key storage

This library exposes `handle_request(&[u8]) -> Vec<u8>` for the desktop host's
isolated credential helper. It stores one lowercase hex-encoded 32-byte AES key
per validated scope under `FerroUI.ToDo.MicrosoftCache.v1`. It never receives
MSAL token-cache blobs and does not log requests, secrets, or platform errors.

Requests are JSON, capped at 4096 bytes, with exactly one of these shapes:

```json
{"operation":"read","scope":"<64 lowercase hex characters>"}
{"operation":"write","scope":"<64 lowercase hex characters>","value":"<64 lowercase hex characters>"}
{"operation":"delete","scope":"<64 lowercase hex characters>"}
```

Responses are JSON capped at 1024 bytes: `ok`, `missing`, `invalid`, or
`unavailable` in the `status` field. Only a successful read includes a `value`.
Writing succeeds only after reading back the exact key; deleting a missing key
succeeds. An invalid stored value is unavailable and is never returned.

The pinned `keyring` v1 adapter uses Windows Credential Manager, macOS Keychain
Services, or Linux Secret Service. Unsupported platforms and inaccessible
stores fail without a plaintext fallback. The desktop caller supplies a
process deadline and private pipes. Unit tests inject a store and do not use
the user's OS vault; actual vault acceptance requires platform testing.

See [native account setup](../../docs/NATIVE-MICROSOFT.md) and the upstream
[keyring adapter](https://docs.rs/keyring/4.2.0/keyring/v1/index.html).

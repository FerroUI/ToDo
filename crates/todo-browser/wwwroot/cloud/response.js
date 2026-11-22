// Enforce the limit while streaming; Content-Length may be missing or wrong.
export async function boundedBlob(response, maxBytes = 25 * 1024 * 1024) {
  if (!Number.isSafeInteger(maxBytes) || maxBytes < 1 || maxBytes > 256 * 1024 * 1024) throw new Error("Invalid Microsoft download size limit.");
  const limitError = () => new Error(`Microsoft download exceeds the ${Math.ceil(maxBytes / 1024 / 1024)} MiB limit.`);
  const length = Number(response.headers.get("Content-Length"));
  if (Number.isFinite(length) && length > maxBytes) {
    await response.body?.cancel().catch(() => {});
    throw limitError();
  }
  if (!response.body?.getReader) {
    const blob = await response.blob();
    if (blob.size > maxBytes) throw limitError();
    return blob;
  }
  const reader = response.body.getReader(), chunks = [];
  let size = 0;
  try {
    while (true) {
      const { value, done } = await reader.read();
      if (done) break;
      size += value.byteLength;
      if (size > maxBytes) { await reader.cancel().catch(() => {}); throw limitError(); }
      chunks.push(value);
    }
    return new Blob(chunks, { type: response.headers.get("Content-Type") || "application/octet-stream" });
  } finally { reader.releaseLock(); }
}

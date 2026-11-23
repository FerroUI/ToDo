// Match todo-domain::backgrounds: these values travel inside account snapshots,
// portable backups and the app-owned OneDrive presentation replica.
const PREFIX = "data:image/jpeg;base64,", MAX_BYTES = 192 * 1024, MAX_SIDE = 1200;
export function validBackground(value) {
  if (typeof value !== "string" || !value) return false;
  if (!value.startsWith("data:")) return value.length <= 128 && !/[\u0000-\u001f\u007f-\u009f]/.test(value);
  if (!value.startsWith(PREFIX)) return false;
  const encoded = value.slice(PREFIX.length);
  if (encoded.length > Math.ceil(MAX_BYTES / 3) * 4 || !/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(encoded)) return false;
  let decoded;
  try { decoded = atob(encoded); if (btoa(decoded) !== encoded) return false; } catch { return false; }
  if (decoded.length > MAX_BYTES || decoded.charCodeAt(0) !== 0xff || decoded.charCodeAt(1) !== 0xd8) return false;
  const byte = index => decoded.charCodeAt(index), word = index => byte(index) * 256 + byte(index + 1);
  let position = 2;
  while (position < decoded.length) {
    if (byte(position) !== 0xff) return false;
    while (byte(position) === 0xff) position++;
    const marker = byte(position++);
    if (marker === 0xda || marker === 0xd9) return false;
    if (marker === 1 || marker >= 0xd0 && marker <= 0xd7) continue;
    const length = word(position);
    if (!Number.isFinite(length) || length < 2 || position + length > decoded.length) return false;
    if (marker >= 0xc0 && marker <= 0xcf && ![0xc4, 0xc8, 0xcc].includes(marker)) {
      if (length < 8) return false;
      const height = word(position + 3), width = word(position + 5);
      return width > 0 && height > 0 && width <= MAX_SIDE && height <= MAX_SIDE;
    }
    position += length;
  }
  return false;
}

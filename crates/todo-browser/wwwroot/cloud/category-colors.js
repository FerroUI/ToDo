// Graph defines semantic presets, not universal RGB values: Outlook clients
// choose their own palette. These swatches use the app's Fluent-style palette.
const COLORS = Object.freeze([
  ["Red", "#D13438"], ["Orange", "#F7630C"], ["Brown", "#8E562E"], ["Yellow", "#FFB900"], ["Green", "#498205"],
  ["Teal", "#038387"], ["Olive", "#8C8C14"], ["Blue", "#0078D4"], ["Purple", "#8764B8"], ["Cranberry", "#C239B3"],
  ["Steel", "#567C73"], ["Dark steel", "#365D54"], ["Gray", "#8A8886"], ["Dark gray", "#605E5C"], ["Black", "#323130"],
  ["Dark red", "#A4262C"], ["Dark orange", "#CA5010"], ["Dark brown", "#603D30"], ["Dark yellow", "#986F0B"], ["Dark green", "#0B6A0B"],
  ["Dark teal", "#004B50"], ["Dark olive", "#6B6B00"], ["Dark blue", "#004E8C"], ["Dark purple", "#5C2D91"], ["Dark cranberry", "#881798"],
]);

export function categoryColor(value) {
  const preset = typeof value === "string" ? value.toLowerCase() : "none";
  const match = /^preset(\d|1\d|2[0-4])$/.exec(preset);
  const [color_name, color] = match ? COLORS[Number(match[1])] : ["No color", "#8A8886"];
  return { preset: match ? preset : "none", color_name, color };
}

export function normalizeCategories(items) {
  if (!Array.isArray(items) || items.length > 4096) throw new Error("Microsoft returned an invalid Outlook category catalogue.");
  const ids = new Set(), names = new Set(), categories = [];
  for (const value of items) {
    if (!value || typeof value.id !== "string" || !value.id || value.id.length > 1024
      || typeof value.displayName !== "string" || !value.displayName.trim() || value.displayName.length > 1024 || ids.has(value.id)) {
      throw new Error("Microsoft returned an invalid Outlook category.");
    }
    ids.add(value.id);
    // Task category membership uses the original display name, not the ID.
    const name = value.displayName, canonical = name.toLocaleLowerCase();
    if (names.has(canonical)) continue;
    names.add(canonical);
    categories.push({ id: value.id, name, ...categoryColor(value.color) });
  }
  return categories.sort((a, b) => a.name.localeCompare(b.name));
}

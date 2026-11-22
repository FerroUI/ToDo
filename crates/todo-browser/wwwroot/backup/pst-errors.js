export const PST_RESOURCE_ERROR = "The Outlook PST could not be parsed within this browser's memory or processing limits. Export a smaller task-only PST in Outlook and try again. Your current tasks were kept.";

export function pstParserError(error) {
  if (error instanceof WebAssembly.RuntimeError || error instanceof RangeError) return PST_RESOURCE_ERROR;
  return String(error?.message || error || "The Outlook PST could not be read.");
}

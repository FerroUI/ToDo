// Parsing an imported mailbox never runs on the UI thread. This module loads
// only the small interchange WASM build, not FerroUI or a Microsoft session.
import initialize, { importPstData } from "../interop/todo_interop_wasm.js";
import { pstParserError } from "./pst-errors.js";

self.onmessage = async ({ data }) => {
  try {
    await initialize();
    const result = importPstData(new Uint8Array(data.bytes), data.current, data.today);
    self.postMessage({ result });
  } catch (error) {
    self.postMessage({ error: pstParserError(error) });
  } finally {
    self.close();
  }
};

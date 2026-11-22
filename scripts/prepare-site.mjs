import path from "node:path";
import { prepareSite } from "./site-build.mjs";
import { copySiteFonts } from "./site-fonts.mjs";

const root = path.resolve(process.argv[2] ?? "target/browser/todo-browser");
await copySiteFonts(root);
const result = await prepareSite(root);
console.log(`Prepared ${result.assets.length} offline resources (${result.version}).`);

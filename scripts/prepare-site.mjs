import path from "node:path";
import { prepareSite } from "./site-build.mjs";

const result = await prepareSite(path.resolve(process.argv[2] ?? "target/browser/todo-browser"));
console.log(`Prepared ${result.assets.length} offline resources (${result.version}).`);

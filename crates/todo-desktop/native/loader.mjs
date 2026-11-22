const web = new URL("../web/", import.meta.url);
export async function resolve(specifier, context, next) {
  if (context.parentURL && (specifier.startsWith(".") || specifier.startsWith("file:"))) {
    const target = new URL(specifier, context.parentURL);
    if (target.href === new URL("cloud/auth.js", web).href) return { url: new URL("./auth.mjs", import.meta.url).href, shortCircuit: true };
    if (target.href === new URL("attachments.js", web).href) return { url: new URL("./files.mjs", import.meta.url).href, shortCircuit: true };
  }
  return next(specifier, context);
}

import { writeFile } from "node:fs/promises";
import path from "node:path";

const stackOf = error => error?.stack || String(error);

async function failureArtifacts(ui, output, name, error) {
  const stem = path.join(output, `failure-${name}`);
  const writes = await Promise.allSettled([
    Promise.resolve().then(() => ui.page.screenshot(`${stem}.png`)),
    Promise.resolve().then(() => ui.snapshot()).then(view => writeFile(`${stem}.json`, JSON.stringify(view))),
    writeFile(`${stem}.txt`, stackOf(error)),
  ]);
  return writes.flatMap((result, index) => result.status === "rejected"
    ? [{ artifact: ["screenshot", "snapshot", "stack"][index], error: stackOf(result.reason) }] : []);
}

// A failed review keeps its original evidence and document. Only ordinary navigation
// resets transient controls; later scenarios must still satisfy every assertion.
export async function runReviewGroups(ui, output, groups, { recover, reportName = "reviews.json" } = {}) {
  const failures = [], results = [];
  let needsReload = false;
  for (const { name, run } of groups) {
    const precedingFailures = failures.map(failure => failure.name);
    let phase = "review";
    try {
      if (needsReload) {
        phase = "recovery";
        if (recover) {
          await recover();
        } else {
          await ui.navigate("Page.reload", { ignoreCache: true }); // navigate includes ready().
          await ui.resize(1280, 800);
        }
      }
      phase = "review";
      console.log(`review: ${name}${precedingFailures.length
        ? ` (persisted data retained after: ${precedingFailures.join(", ")})` : ""}`);
      await run();
      ui.healthy();
      results.push({ name, status: "passed", precedingFailures });
      needsReload = false;
      console.log(`ok: review ${name}`);
    } catch (error) {
      console.error(`not ok: review ${name} (${phase})\n${stackOf(error)}`);
      const diagnostics = await failureArtifacts(ui, output, name, error);
      failures.push({ name, phase, error });
      results.push({ name, status: phase === "recovery" ? "blocked" : "failed",
        phase, precedingFailures, stack: stackOf(error), diagnostics });
      needsReload = true;
    }
  }
  try {
    await writeFile(path.join(output, reportName), JSON.stringify(results, null, 2));
  } catch (error) {
    console.error(`not ok: writing review results\n${stackOf(error)}`);
    failures.push({ name: "review-report", phase: "report", error });
  }
  return failures;
}

import { writeFile } from "node:fs/promises";
import path from "node:path";
import { withDeadline } from "./deadline.mjs";

const stackOf = error => error?.stack || String(error);

async function failureArtifacts(ui, output, name, error) {
  const stem = path.join(output, `failure-${name}`);
  const artifacts = ["screenshot", "snapshot", "stack"];
  const operations = [
    withDeadline(() => ui.page.screenshot(`${stem}.png`), `${name} diagnostic screenshot`),
    withDeadline(async () => writeFile(`${stem}.json`, JSON.stringify(await ui.snapshot())), `${name} diagnostic snapshot`),
    withDeadline(() => writeFile(`${stem}.txt`, stackOf(error)), `${name} original failure`),
  ];
  if (error?.inputDiagnostics) {
    artifacts.push("input");
    operations.push(withDeadline(() => writeFile(`${stem}.input.json`, JSON.stringify(error.inputDiagnostics, null, 2)),
      `${name} input diagnostics`));
  }
  const writes = await Promise.allSettled(operations);
  return writes.flatMap((result, index) => result.status === "rejected"
    ? [{ artifact: artifacts[index], error: stackOf(result.reason) }] : []);
}

// A failed review keeps its original evidence and document. Only ordinary navigation
// resets transient controls; later scenarios must still satisfy every assertion.
export async function runReviewGroups(ui, output, groups, { recover, reportName = "reviews.json" } = {}) {
  const failures = [], results = [];
  let reportFailed = false;
  const persist = async () => {
    try {
      await withDeadline(() => writeFile(path.join(output, reportName), JSON.stringify(results, null, 2)), "write review progress");
    } catch (error) {
      if (!reportFailed) {
        console.error(`not ok: writing review results\n${stackOf(error)}`);
        failures.push({ name: "review-report", phase: "report", error });
        reportFailed = true;
      }
    }
  };
  let needsReload = false;
  for (const { name, run } of groups) {
    const precedingFailures = failures.map(failure => failure.name);
    let phase = "review";
    const result = { name, status: needsReload ? "recovering" : "running", precedingFailures };
    results.push(result);
    await persist();
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
      if (needsReload) { result.status = "running"; await persist(); }
      console.log(`review: ${name}${precedingFailures.length
        ? ` (persisted data retained after: ${precedingFailures.join(", ")})` : ""}`);
      await run();
      ui.healthy();
      result.status = "passed";
      needsReload = false;
      console.log(`ok: review ${name}`);
    } catch (error) {
      console.error(`not ok: review ${name} (${phase})\n${stackOf(error)}`);
      const diagnostics = await failureArtifacts(ui, output, name, error);
      failures.push({ name, phase, error });
      Object.assign(result, { status: phase === "recovery" ? "blocked" : "failed",
        phase, precedingFailures, stack: stackOf(error), diagnostics });
      needsReload = true;
    }
    await persist();
  }
  return failures;
}

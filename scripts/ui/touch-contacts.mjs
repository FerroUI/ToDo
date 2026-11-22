import assert from "node:assert/strict";
import { reveal } from "./touch-actions.mjs";
import { center, named, noDetails } from "./touch-input.mjs";
import { noTransient, topTasks } from "./touch-combination-helpers.mjs";

export async function verifyContactRecovery(ctx) {
  const { touch, page, saved, control } = ctx;
  const tasks = await topTasks(ctx);
  const baseline = await saved();
  // One sequence adds a finger after recognition; the other starts both contacts
  // in one protocol event. Lift each identity first, keeping the other alive.
  for (const released of [1, 2]) {
    const first = center(await ctx.row(tasks[1].title));
    const second = center(await ctx.row(tasks[2].title));
    if (released === 1) {
      await touch.start(first);
      await touch.move({ x: first.x - 90, y: first.y });
      await control(named("SwipeDelete"), "recognized swipe before adding a finger");
      await touch.addContact(second);
    } else {
      await touch.startContacts([first, second]);
    }
    await touch.releaseContact(released);
    const remaining = released === 1 ? 2 : 1;
    const point = remaining === 1 ? first : second;
    await touch.moveContact(remaining, { x: point.x - 110, y: point.y });
    await noTransient(ctx, "a surviving quarantined finger cannot restart a gesture");
    await touch.releaseContact(remaining);
    await noDetails(ctx, "partial release cannot activate either row");
    assert.deepEqual(await saved(), baseline, "both contact-release orders preserve saved data");
    await reveal(ctx, tasks[0].title, -1);
    await control(named("SwipeDelete"), "a new contact works after both fingers lift");
    await page.press("Escape");
    await noTransient(ctx, "the fresh recovered rail closes normally");
  }
  console.log("ok: simultaneous contacts, both partial-lift orders, surviving-finger quarantine, and fresh-contact recovery");
}

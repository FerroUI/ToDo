import assert from "node:assert/strict";

const button = label => element => element.type.endsWith("Button") && element.automationName === label;

// Use the retained native editor: category names are individual values, not CSV or hashtags.
export async function verifyCategories(ui, taskId) {
  const { page, saved, resize, control, edit, clickControl, until, capture } = ui;
  const task = async () => (await saved()).tasks.find(task => task.id === taskId);
  const before = (await task()).tags;
  const category = "#Release, 2027";
  await resize(1280, 844);
  await page.wheel(1200, 600, 0, 1800);
  await edit("NewCategory", category, true);
  await until(async () => (await task()).tags.includes(category), "a category keeps its comma and leading #");
  await page.wheel(1200, 600, 0, 400);
  await edit("NewCategory", category.toLowerCase(), true);
  assert.equal((await task()).tags.filter(value => value.toLowerCase() === category.toLowerCase()).length, 1,
    "case-insensitive duplicate entry preserves the existing category name");
  for (const [width, label] of [[1280, "wide"], [320, "narrow"]]) {
    await resize(width, 844);
    await page.wheel(width - 60, 600, 0, 1800);
    const field = await control(element => element.name === "NewCategory", "category editor");
    const remove = await control(button(`Remove category ${category}`), "category remove action");
    for (const item of [field, remove]) {
      assert(item.x >= -1 && item.x + item.width <= width + 1, `category control overflows ${width}px`);
      assert(item.height >= 32, "category actions retain a usable input area");
    }
    assert.equal(field.text, "", "submitting a category clears its draft");
    await capture(`categories-${label}`);
  }
  await clickControl(button(`Remove category ${category}`), "remove the temporary category");
  await until(async () => !(await task()).tags.includes(category), "category removal is saved");
  assert.deepEqual((await task()).tags, before, "category review preserves all original names");
  const catalogue = [{id:"review-blue",name:"Outlook catalogue review",color:"#0078D4",color_name:"Blue"}];
  await page.evaluate(`todoApp.outlookCategories(${JSON.stringify(JSON.stringify(catalogue))})`);
  try {
    for(const [width,label] of [[1280,"wide"],[320,"narrow"]]) {
      await resize(width,844);await page.wheel(width-60,600,0,1800);
      await clickControl(button("Choose Outlook categories"),"native Outlook category picker");
      const action=`${width===1280?"Add":"Remove"} category Outlook catalogue review`;
      const item=await control(element=>element.type.endsWith("MenuItem")&&element.automationName===action,action);
      assert(item.x>=-1&&item.x+item.width<=width+1,"category picker remains inside the viewport");
      await capture(`outlook-category-picker-${label}`);
      await clickControl(element=>element.type.endsWith("MenuItem")&&element.automationName===action,action);
      await until(async()=> (await task()).tags.includes(catalogue[0].name)===(width===1280),"catalogue selection updates the task through the native menu");
    }
    assert.deepEqual((await task()).tags,before,"catalogue review restores original categories");
  } finally { await page.evaluate("todoApp.outlookCategories('[]')"); }
}

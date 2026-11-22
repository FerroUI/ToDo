import assert from "node:assert/strict";

export function one(elements, predicate, label) {
  const matches = elements.filter(predicate);
  assert.equal(matches.length, 1, `Expected one rendered ${label}, found ${matches.length}`);
  return matches[0];
}

export const centerX = element => element.x + element.width / 2;
export const centerY = element => element.y + element.height / 2;
export const right = element => element.x + element.width;
export const bottom = element => element.y + element.height;

export function close(actual, expected, label, tolerance = 1) {
  assert(Math.abs(actual - expected) <= tolerance,
    `${label}: expected ${expected.toFixed(2)}, received ${actual.toFixed(2)}`);
}

export function sameBounds(actual, expected, label) {
  for (const key of ["x", "y", "width", "height"]) close(actual[key], expected[key], `${label}: ${key}`);
}

export function textPresenter(view, name) {
  return one(view.elements, element => element.name === "PART_TextPresenter"
    && element.textBoxName === name, `${name} native text presenter`);
}

export function editorFits(view, name) {
  const field = one(view.elements, element => element.name === name, name);
  const text = textPresenter(view, name);
  assert(text.x >= field.x - 1 && right(text) <= right(field) + 1,
    `${name} native content viewport must remain within the editor`);
  assert(!view.elements.some(element => element.name === "DeleteButton" && element.textBoxName === name),
    `${name} must not reserve a second native clear affordance`);
  return field;
}

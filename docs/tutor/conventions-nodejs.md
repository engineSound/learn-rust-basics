## File conventions (Node.js track)

- `starter`: the complete content of `exercise.js`, an ES module. Export each function with a JSDoc comment showing an example call and result, and a body of exactly `throw new Error('TODO: <function name>');`.
- `tests`: the complete content of `exercise.test.js`, using only `import { test } from 'node:test';`, `import assert from 'node:assert/strict';` and `import { ... } from './exercise.js';`. One behaviour per `test(...)`.
- `solution`: the complete content of a correct `exercise.js`.
- Node 22, no npm packages.
- In `explanation`, ```run and ```predict blocks are ES modules run with `node`: print with `console.log`.
- Test names are plain sentences a learner understands ("0% is low"), and use `assert.equal` / `assert.deepEqual` so failures show the expected and actual values.

// This file is used for compiling tests with webpack into one file for using with karma
require('./bootstrap');

// noinspection JSUnresolvedFunction
const testsContext = require.context('../../../test', true, /^.+\.spec\.js$/);

let batch;

if (process.env.BROWSER_TEST_BATCH_INDEX !== '0'
  && process.env.BROWSER_TEST_BATCH_TOTAL !== '0') {
  const batchTotal = parseInt(process.env.BROWSER_TEST_BATCH_TOTAL, 10);
  const batchIndex = parseInt(process.env.BROWSER_TEST_BATCH_INDEX, 10);

  const files = testsContext.keys();
  const batchSize = Math.ceil(files.length / batchTotal);

  const batches = [];
  for (let i = 0; i < files.length; i += batchSize) {
    batches.push(files.slice(i, i + batchSize));
  }

  batch = batches[batchIndex] || [];

  console.log('Selected tests', batch);
}

function filterBath(path) {
  if (batch === undefined) {
    return true;
  }

  return batch.includes(path);
}

testsContext.keys()
  // Ignore proofs.spec.js because it uses Merk native Node.JS module
  .filter((path) => !path.includes('proofs.spec.js'))
  .filter((path) => !path.includes('waitForStateTransitionResult.spec.js'))
  .filter(filterBath)
  .forEach(testsContext);

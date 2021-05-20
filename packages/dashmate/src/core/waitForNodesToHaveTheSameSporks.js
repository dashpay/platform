const isEqual = require('lodash.isequal');

/**
 * @param {CoreService[]} coreServices
 * @return {Promise<boolean>}
 */
async function checkSporksAreTheSame(coreServices) {
  const { result: initialSporks } = await coreServices[0].getRpcClient().spork('show');

  for (const coreService of coreServices.slice(1)) {
    const { result: sporks } = await coreService.getRpcClient().spork('show');

    if (!isEqual(initialSporks, sporks)) {
      return false;
    }
  }

  return true;
}

/**
 * @param {CoreService[]} coreServices
 * @param {number} [timeout]
 * @return {Promise<void>}
 */
async function waitForNodesToHaveTheSameSporks(coreServices, timeout = 30000) {
  const deadline = Date.now() + timeout;

  let isReady = false;

  while (!isReady) {
    isReady = await checkSporksAreTheSame(coreServices);

    if (Date.now() > deadline) {
      throw new Error(`Syncing sporks deadline of ${timeout} exceeded`);
    }
  }
}

module.exports = waitForNodesToHaveTheSameSporks;

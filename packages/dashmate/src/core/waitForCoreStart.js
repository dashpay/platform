import wait from '../util/wait.js';

/**
 * Wait for Core to start
 *
 * @typedef {waitForCoreStart}
 * @param {CoreService} coreService
 * @param {number} [maxRetries=120] ~2 minutes
 * @param {number} [delayMs=1000]
 * @return {Promise<void>}
 */
export default async function waitForCoreStart(coreService, maxRetries = 120, delayMs = 1000) {
  let retries = 0;
  let isReady = false;

  do {
    try {
      // just any random request
      await coreService.getRpcClient().ping();

      isReady = true;
    } catch (e) {
      // just wait 1 second before next try
      await wait(delayMs);
      ++retries;
    }
  } while (!isReady && retries < maxRetries);

  if (!isReady) {
    throw new Error('Could not connect to Core RPC');
  }
}

import wait from '../util/wait.js';

/**
 * Wait for Core to start
 *
 * @typedef {waitForCoreStart}
 * @param {CoreService} coreService
 * @return {Promise<void>}
 */
export default async function waitForCoreStart(coreService) {
  let retries = 0;
  let isReady = false;
  const maxRetries = 120; // ~2 minutes

  do {
    try {
      // just any random request
      await coreService.getRpcClient().ping();

      isReady = true;
    } catch (e) {
      // just wait 1 second before next try
      await wait(1000);
      ++retries;
    }
  } while (!isReady && retries < maxRetries);

  if (!isReady) {
    throw new Error('Could not connect to Core RPC');
  }
}

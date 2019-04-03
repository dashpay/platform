/* eslint-disable no-await-in-loop */
const wait = require('../utils/wait');

/**
 * Await Drive instance to finish syncing
 *
 * @param {RpcClient} driveClient
 * @returns {Promise<void>}
 */
async function isDriveSynced(driveClient) {
  let finished = false;
  while (!finished) {
    try {
      const { result: syncInfo } = await driveClient.request('getSyncInfo', []);

      if (syncInfo.status === 'synced') {
        finished = true;
        await wait(1000);
      } else {
        await wait(1000);
      }
    } catch (e) {
      await wait(1000);
    }
  }
}

module.exports = isDriveSynced;

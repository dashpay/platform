/* eslint-disable no-param-reassign */
const logger = require('../../../../logger');
const sleep = require('../../../../utils/sleep');

function onStreamEnd(workerInstance, resolve) {
  const endStream = () => {
    logger.silly('TransactionSyncStreamWorker - end stream on request');
    workerInstance.stream = null;
    resolve(workerInstance.hasReachedGapLimit);
  };

  const tryEndStream = async () => {
    if (Object.keys(workerInstance.pendingRequest).length !== 0) {
      await sleep(200);
      return tryEndStream();
    }
    return endStream();
  };

  tryEndStream();
}
module.exports = onStreamEnd;

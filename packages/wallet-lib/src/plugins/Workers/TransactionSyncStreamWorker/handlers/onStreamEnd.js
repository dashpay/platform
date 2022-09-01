/* eslint-disable no-param-reassign */
const logger = require('../../../../logger');

function onStreamEnd(workerInstance, resolve) {
  logger.silly('TransactionSyncStreamWorker - end stream on request');
  workerInstance.stream = null;
  resolve(workerInstance.hasReachedGapLimit);
}
module.exports = onStreamEnd;

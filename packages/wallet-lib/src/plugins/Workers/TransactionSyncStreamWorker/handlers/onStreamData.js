/* eslint-disable no-param-reassign */
const logger = require('../../../../logger');
const Job = require('../../../../utils/Queue/Job');

function onStreamData(self, data) {
  logger.silly('TransactionSyncStreamWorker - received chunks waiting for processing');
  self.chunksQueue.enqueueJob(new Job(null, () => self.processChunks(data)));
}

module.exports = onStreamData;

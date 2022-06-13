/* eslint-disable no-param-reassign */
const logger = require('../../../../logger');
const Job = require('../../../../utils/Queue/Job');

function onStreamData(self, data) {
  logger.silly('TransactionSyncStreamWorker - received chunks waiting for processing');
  // TODO: refactor to distinguish between type of chunk (transactions/merkleBlocks/Instant locks)
  // otherwise merkle block might hang in the queue forever
  self.chunksQueue.enqueueJob(new Job(null, () => self.processChunks(data)));
}

module.exports = onStreamData;

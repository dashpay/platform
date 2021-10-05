const logger = require('../../../../logger');

function onStreamError(error, reject) {
  logger.silly('TransactionSyncStreamWorker - end stream on error');
  reject(error);
}
module.exports = onStreamError;

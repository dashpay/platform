const logger = require('../../../../logger');

function onStreamError(error, reject) {
  logger.silly('TransactionSyncStreamWorker - end stream on error');
  logger.silly(error.message);
  reject(error);
}
module.exports = onStreamError;

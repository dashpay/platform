const { utils } = require('jayson');

const createError = require('../api/jsonRpc/createError');

/**
 * Returns error until initial sync process is not completed
 *
 * @param {Function} inSynced
 * @param {getSyncInfo} getSyncInfo
 * @param {SyncStateRepositoryChangeListener} stateRepositoryChangeListener
 * @param {number} checkInterval
 */
function getCheckSyncHttpMiddleware(
  inSynced,
  getSyncInfo,
  stateRepositoryChangeListener,
  checkInterval,
) {
  // Get sync state
  let syncInfo;
  let error;

  inSynced(getSyncInfo, stateRepositoryChangeListener, checkInterval).then((info) => {
    syncInfo = info;
  }).catch((e) => {
    error = e;
  });

  // Handle RPC requests nad respond error if state unresolved
  return (req, res, next) => {
    if (error) {
      throw error;
    }
    if (syncInfo || !utils.Request.isValidRequest(req.body) || !req.body.id) {
      next();
      return;
    }

    const requests = utils.Request.isBatch(req.body) ? req.body : [req.body];
    const errorResponse = createError(100, 'Initial sync in progress');
    const responses = requests.map(r => utils.response(errorResponse, null, r.id));

    const requestsStringify = JSON.stringify(requests);
    const responsesStringify = JSON.stringify(responses[0]);
    res.end(utils.Request.isBatch(req.body) ? requestsStringify : responsesStringify);
  };
}

module.exports = getCheckSyncHttpMiddleware;

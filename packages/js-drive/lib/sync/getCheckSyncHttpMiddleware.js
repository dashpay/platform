const { utils } = require('jayson');

const createError = require('../api/jsonRpc/createError');

/**
 * Returns error until initial sync process is not completed
 *
 * @param {Function} inSynced
 * @param {RpcClient} rpcClient
 * @param {SyncStateRepositoryChangeListener} stateRepositoryChangeListener
 * @param {number} checkInterval
 */
function getCheckSyncHttpMiddleware(
  inSynced,
  rpcClient,
  stateRepositoryChangeListener,
  checkInterval,
) {
  // Get sync state
  let syncState;
  let error;

  inSynced(rpcClient, stateRepositoryChangeListener, checkInterval).then((state) => {
    syncState = state;
  }).catch((e) => {
    error = e;
  });

  // Handle RPC requests nad respond error if state unresolved
  return (req, res, next) => {
    if (error) {
      throw error;
    }
    utils.parseBody(req, null, (err, request) => {
      if (err) {
        next(err);
        return;
      }

      if (syncState || !utils.Request.isValidRequest(request) || !request.id) {
        req.body = request;
        next();
        return;
      }

      const requests = utils.Request.isBatch(request) ? request : [request];
      const errorResponse = createError(100, 'Initial sync in progress');
      const responses = requests.map(r => utils.response(errorResponse, null, r.id));

      const requestsStringify = JSON.stringify(requests);
      const responsesStringify = JSON.stringify(responses[0]);
      res.end(utils.Request.isBatch(request) ? requestsStringify : responsesStringify);
    });
  };
}

module.exports = getCheckSyncHttpMiddleware;

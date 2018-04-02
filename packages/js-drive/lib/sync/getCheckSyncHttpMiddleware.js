const { utils, server } = require('jayson');

const createError = server.prototype.error.bind(server);

/**
 * Returns error until initial sync process is not completed
 *
 * @param {Function} inSynced
 * @param {RpcClient} rpcClient
 * @param {SyncStateRepositoryChangeListener} stateRepositoryChangeListener
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

    if (syncState) {
      next();

      return;
    }

    utils.parseBody(req, (err, request) => {
      if (err || !utils.Request.isValidRequest(request) || !request.id) {
        next();

        return;
      }

      const requests = utils.Request.isBatch(request) ? request : [request];
      const errorResponse = createError(100, 'Initial sync in progress');
      const responses = requests.map(r => utils.response(errorResponse, null, r.id));

      res.end(utils.Request.isBatch(request) ? requests : responses[0]);
    });
  };
}

module.exports = getCheckSyncHttpMiddleware;

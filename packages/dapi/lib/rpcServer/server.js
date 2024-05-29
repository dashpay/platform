const jayson = require('jayson/promise');
const errorHandlerDecorator = require('./errorHandlerDecorator');

const getBestBlockHash = require('./commands/getBestBlockHash');
const getBlockHash = require('./commands/getBlockHash');
const getMnListDiff = require('./commands/getMnListDiff');

// Following commands are not implemented yet:
// const getVersion = require('./commands/getVersion');

const createCommands = (dashcoreAPI) => ({
  getBestBlockHash: getBestBlockHash(dashcoreAPI),
  getBlockHash: getBlockHash(dashcoreAPI),
  getMnListDiff: getMnListDiff(dashcoreAPI),
});

/**
  * Starts RPC server
 *  @param options
  * @param {number} options.port - port to listen for incoming RPC connections
  * @param {string} options.networkType
  * @param {object} options.dashcoreAPI
  * @param {AbstractDriveAdapter} options.driveAPI - Drive api adapter
  * @param {object} options.tendermintRpcClient
  * @param {DashPlatformProtocol} options.dpp
  * @param {object} options.log
 */
const start = ({
  port,
  dashcoreAPI,
  logger,
}) => {
  const commands = createCommands(
    dashcoreAPI,
  );
  /*
  Decorate all commands with decorator that will intercept errors and format
  them before passing to user.
  */
  Object.keys(commands).forEach((commandName) => {
    commands[commandName] = errorHandlerDecorator(commands[commandName], logger);
  });

  const server = jayson.server(commands);
  server.http().listen(port);
};

module.exports = {
  createCommands,
  start,
};

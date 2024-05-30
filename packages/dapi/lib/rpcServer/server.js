const jayson = require('jayson/promise');
const errorHandlerDecorator = require('./errorHandlerDecorator');

const getBestBlockHash = require('./commands/getBestBlockHash');
const getBlockHash = require('./commands/getBlockHash');

// Following commands are not implemented yet:
// const getVersion = require('./commands/getVersion');

const createCommands = (dashcoreAPI, coreZmqClient) => ({
  getBestBlockHash: getBestBlockHash(dashcoreAPI, coreZmqClient),
  getBlockHash: getBlockHash(dashcoreAPI),
});

/**
  * Starts RPC server
 *  @param options
  * @param {number} options.port - port to listen for incoming RPC connections
  * @param {object} options.dashcoreAPI
  * @param {Logger} options.logger
  * @param {ZmqClient} options.coreZmqClient
 */
const start = ({
  port,
  dashcoreAPI,
  logger,
  coreZmqClient,
}) => {
  const commands = createCommands(
    dashcoreAPI,
    coreZmqClient,
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

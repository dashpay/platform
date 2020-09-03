const jayson = require('jayson/promise');
const { isRegtest, isDevnet } = require('../utils');
const errorHandlerDecorator = require('./errorHandlerDecorator');

const getBestBlockHash = require('./commands/getBestBlockHash');
const getBlockHash = require('./commands/getBlockHash');
const getMnListDiff = require('./commands/getMnListDiff');
const generateToAddress = require('./commands/generateToAddress');

// Following commands are not implemented yet:
// const getVersion = require('./commands/getVersion');

const createCommands = dashcoreAPI => ({
  getBestBlockHash: getBestBlockHash(dashcoreAPI),
  getBlockHash: getBlockHash(dashcoreAPI),
  getMnListDiff: getMnListDiff(dashcoreAPI),
});

const createRegtestCommands = dashcoreAPI => ({
  generateToAddress: generateToAddress(dashcoreAPI),
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
  networkType,
  dashcoreAPI,
  log,
}) => {
  const commands = createCommands(
    dashcoreAPI,
  );

  const areRegtestCommandsEnabled = isRegtest(networkType) || isDevnet(networkType);

  const allCommands = areRegtestCommandsEnabled
    ? Object.assign(commands, createRegtestCommands(dashcoreAPI))
    : commands;

  /*
  Decorate all commands with decorator that will intercept errors and format
  them before passing to user.
  */
  Object.keys(allCommands).forEach((commandName) => {
    allCommands[commandName] = errorHandlerDecorator(allCommands[commandName], log);
  });

  const server = jayson.server(allCommands);
  server.http().listen(port);
};

module.exports = {
  createCommands,
  start,
};

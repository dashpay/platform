const jayson = require('jayson/promise');
const { isRegtest, isDevnet } = require('../utils');
const Validator = require('../utils/Validator');
const errorHandlerDecorator = require('./errorHandlerDecorator');

const getIdentityArgsSchema = require('./commands/platform/schemas/getIdentity');
const getDocumentsArgsSchema = require('./commands/platform/schemas/getDocuments');
const getDataContractArgsSchema = require('./commands/platform/schemas/getDataContract');
const applyIdentityArgsSchema = require('./commands/platform/schemas/applyStateTransition');

const getBestBlockHash = require('./commands/getBestBlockHash');
const getBlockHash = require('./commands/getBlockHash');
const getMnListDiff = require('./commands/getMnListDiff');
const getUTXO = require('./commands/getUTXO');
const generateToAddress = require('./commands/generateToAddress');
const getAddressSummary = require('./commands/getAddressSummary');

const getIdentityHandlerFactory = require('./commands/platform/getIdentityHandlerFactory');
const applyStateTransitionHandlerFactory = require('./commands/platform/applyStateTransitionHandlerFactory');
const getDataContractHandlerFactory = require('./commands/platform/getDataContractHandlerFactory');
const getDocumentsHandlerFactory = require('./commands/platform/getDocumentsHandlerFactory');

const handleAbciResponse = require('../grpcServer/handlers/handleAbciResponse');


// Following commands are not implemented yet:
// const getVersion = require('./commands/getVersion');

const createCommands = (
  insightAPI,
  dashcoreAPI,
  driveAPI,
  tendermintRpcClient,
  dpp,
) => ({
  getBestBlockHash: getBestBlockHash(dashcoreAPI),
  getBlockHash: getBlockHash(dashcoreAPI),
  getMnListDiff: getMnListDiff(dashcoreAPI),
  getUTXO: getUTXO(insightAPI),
  getAddressSummary: getAddressSummary(insightAPI),

  getIdentity: getIdentityHandlerFactory(
    tendermintRpcClient, handleAbciResponse, new Validator(getIdentityArgsSchema),
  ),
  applyStateTransition: applyStateTransitionHandlerFactory(
    tendermintRpcClient, handleAbciResponse, new Validator(applyIdentityArgsSchema),
  ),
  getDataContract: getDataContractHandlerFactory(
    driveAPI, dpp, new Validator(getDataContractArgsSchema),
  ),
  getDocuments: getDocumentsHandlerFactory(
    driveAPI, dpp, new Validator(getDocumentsArgsSchema),
  ),

});

const createRegtestCommands = dashcoreAPI => ({
  generateToAddress: generateToAddress(dashcoreAPI),
});

/**
  * Starts RPC server
 *  @param options
  * @param {number} options.port - port to listen for incoming RPC connections
  * @param {string} options.networkType
  * @param {object} options.insightAPI
  * @param {object} options.dashcoreAPI
  * @param {AbstractDriveAdapter} options.driveAPI - Drive api adapter
  * @param {object} options.tendermintRpcClient
  * @param {DashPlatformProtocol} options.dpp
  * @param {object} options.log
 */
const start = ({
  port,
  networkType,
  insightAPI,
  dashcoreAPI,
  driveAPI,
  tendermintRpcClient,
  dpp,
  log,
}) => {
  const commands = createCommands(
    insightAPI,
    dashcoreAPI,
    driveAPI,
    tendermintRpcClient,
    dpp,
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

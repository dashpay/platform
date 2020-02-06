const jayson = require('jayson/promise');
const { isRegtest, isDevnet } = require('../utils');
const Validator = require('../utils/Validator');
const errorHandlerDecorator = require('./errorHandlerDecorator');

const getIdnetityArgsSchema = require('./commands/platform/schemas/getIdentity');
const getDocumentsArgsSchema = require('./commands/platform/schemas/getDocuments');
const getDataContractArgsSchema = require('./commands/platform/schemas/getDataContract');
const applyIdentityArgsSchema = require('./commands/platform/schemas/applyStateTransition');

const estimateFee = require('./commands/estimateFee');
const getAddressSummary = require('./commands/getAddressSummary');
const getAddressTotalReceived = require('./commands/getAddressTotalReceived');
const getAddressTotalSent = require('./commands/getAddressTotalSent');
const getAddressUnconfirmedBalance = require('./commands/getAddressUnconfirmedBalance');
const getBalance = require('./commands/getBalance');
const getBestBlockHash = require('./commands/getBestBlockHash');
const getBestBlockHeight = require('./commands/getBestBlockHeight');
const getBlockHash = require('./commands/getBlockHash');
const getBlocks = require('./commands/getBlocks');
const getHistoricBlockchainDataSyncStatus = require('./commands/getHistoricBlockchainDataSyncStatus');
const getMempoolInfo = require('./commands/getMempoolInfo');
const getMNList = require('./commands/getMNList');
const getMnListDiff = require('./commands/getMnListDiff');
const getPeerDataSyncStatus = require('./commands/getPeerDataSyncStatus');
const getRawBlock = require('./commands/getRawBlock');
const getStatus = require('./commands/getStatus');
const getTransactionById = require('./commands/getTransactionById');
const getTransactionsByAddress = require('./commands/getTransactionsByAddress');
const getUser = require('./commands/getUser');
const getUTXO = require('./commands/getUTXO');
const getBlockHeader = require('./commands/getBlockHeader');
const getBlockHeaders = require('./commands/getBlockHeaders');
const sendRawTransaction = require('./commands/sendRawTransaction');
const sendRawIxTransaction = require('./commands/sendRawIxTransaction');
const generate = require('./commands/generate');
const sendRawTransition = require('./commands/sendRawTransition');
const fetchContract = require('./commands/fetchContract');
const fetchDocuments = require('./commands/fetchDocuments');
const searchUsers = require('./commands/searchUsers');
const getQuorum = require('./commands/getQuorum');

const getIdentityHandlerFactory = require('./commands/platform/getIdentityHandlerFactory');
const applyStateTransitionHandlerFactory = require('./commands/platform/applyStateTransitionHandlerFactory');
const getDataContractHandlerFactory = require('./commands/platform/getDataContractHandlerFactory');
const getDocumentsHandlerFactory = require('./commands/platform/getDocumentsHandlerFactory');

const handleAbciResponse = require('../grpcServer/handlers/handleAbciResponse');

// Following commands are not implemented yet:
// const getVersion = require('./commands/getVersion');

const createCommands = (
  insightAPI, dashcoreAPI, driveAPI, userIndex, tendermintRpcClient, dpp,
) => ({
  estimateFee: estimateFee(insightAPI),
  getAddressSummary: getAddressSummary(insightAPI),
  getAddressTotalReceived: getAddressTotalReceived(insightAPI),
  getAddressTotalSent: getAddressTotalSent(insightAPI),
  getAddressUnconfirmedBalance: getAddressUnconfirmedBalance(insightAPI),
  getBalance: getBalance(insightAPI),
  getBestBlockHash: getBestBlockHash(dashcoreAPI),
  getBestBlockHeight: getBestBlockHeight(dashcoreAPI),
  getBlockHash: getBlockHash(dashcoreAPI),
  getBlocks: getBlocks(insightAPI),
  getHistoricBlockchainDataSyncStatus: getHistoricBlockchainDataSyncStatus(insightAPI),
  getMempoolInfo: getMempoolInfo(dashcoreAPI),
  getMNList: getMNList(insightAPI),
  getMnListDiff: getMnListDiff(dashcoreAPI),
  getPeerDataSyncStatus: getPeerDataSyncStatus(insightAPI),
  getRawBlock: getRawBlock(dashcoreAPI),
  getStatus: getStatus(insightAPI),
  getTransactionById: getTransactionById(insightAPI),
  getTransactionsByAddress: getTransactionsByAddress(insightAPI),
  getUTXO: getUTXO(insightAPI),
  getUser: getUser(dashcoreAPI),
  getBlockHeader: getBlockHeader(dashcoreAPI),
  getBlockHeaders: getBlockHeaders(dashcoreAPI),
  sendRawTransaction: sendRawTransaction(dashcoreAPI),
  sendRawIxTransaction: sendRawIxTransaction(dashcoreAPI),
  getQuorum: getQuorum(dashcoreAPI),

  // Methods that are using Drive
  sendRawTransition: sendRawTransition(dashcoreAPI, driveAPI),
  fetchContract: fetchContract(driveAPI),
  fetchDocuments: fetchDocuments(driveAPI),
  searchUsers: searchUsers(userIndex),

  getIdentity: getIdentityHandlerFactory(
    tendermintRpcClient, handleAbciResponse, new Validator(getIdnetityArgsSchema),
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
  generate: generate(dashcoreAPI),
});

/**
  * Starts RPC server
 *  @param options
  * @param {number} options.port - port to listen for incoming RPC connections
  * @param {string} options.networkType
  * @param {object} options.insightAPI
  * @param {object} options.dashcoreAPI
  * @param {AbstractDriveAdapter} options.driveAPI - Drive api adapter
  * @param {object} options.userIndex
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
  userIndex,
  log,
  tendermintRpcClient,
  dpp,
}) => {
  const commands = createCommands(
    insightAPI,
    dashcoreAPI,
    driveAPI,
    userIndex,
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

const jayson = require('jayson/promise');
const { isRegtest, isDevnet } = require('../utils');
const errorHandlerDecorator = require('./errorHandlerDecorator');

const estimateFee = require('./commands/estimateFee');
const getAddressSummary = require('./commands/getAddressSummary');
const getAddressTotalReceived = require('./commands/getAddressTotalReceived');
const getAddressTotalSent = require('./commands/getAddressTotalSent');
const getAddressUnconfirmedBalance = require('./commands/getAddressUnconfirmedBalance');
const getBalance = require('./commands/getBalance');
const getBestBlockHeight = require('./commands/getBestBlockHeight');
const getBlockHash = require('./commands/getBlockHash');
const getBlocks = require('./commands/getBlocks');
const getHistoricBlockchainDataSyncStatus = require('./commands/getHistoricBlockchainDataSyncStatus');
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
const fetchDapContract = require('./commands/fetchDapContract');
const fetchDapObjects = require('./commands/fetchDapObjects');
const searchUsers = require('./commands/searchUsers');
const loadBloomFilter = require('./commands/loadBloomFilter');
const addToBloomFilter = require('./commands/addToBloomFilter');
const clearBloomFilter = require('./commands/clearBloomFilter');
const getSpvData = require('./commands/getSpvData');
const findDataForBlock = require('./commands/findDataForBlock');
const getQuorum = require('./commands/getQuorum');

// Following commands are not implemented yet:
// const getCurrency = require('./commands/getCurrency');
// const getMNUpdateList = require('./commands/getMNUpdateList');
// const getVersion = require('./commands/getVersion');

const createCommands = (insightAPI, dashcoreAPI, dashDriveAPI, userIndex) => ({
  estimateFee: estimateFee(insightAPI),
  getAddressSummary: getAddressSummary(insightAPI),
  getAddressTotalReceived: getAddressTotalReceived(insightAPI),
  getAddressTotalSent: getAddressTotalSent(insightAPI),
  getAddressUnconfirmedBalance: getAddressUnconfirmedBalance(insightAPI),
  getBalance: getBalance(insightAPI),
  getBestBlockHeight: getBestBlockHeight(dashcoreAPI),
  getBlockHash: getBlockHash(dashcoreAPI),
  getBlocks: getBlocks(insightAPI),
  getHistoricBlockchainDataSyncStatus: getHistoricBlockchainDataSyncStatus(insightAPI),
  getMNList: getMNList(insightAPI),
  getMnListDiff: getMnListDiff(dashcoreAPI),
  getPeerDataSyncStatus: getPeerDataSyncStatus(insightAPI),
  getRawBlock: getRawBlock(insightAPI),
  getStatus: getStatus(insightAPI),
  getTransactionById: getTransactionById(insightAPI),
  getTransactionsByAddress: getTransactionsByAddress(insightAPI),
  getUser: getUser(insightAPI),
  getUTXO: getUTXO(insightAPI),
  getBlockHeader: getBlockHeader(dashcoreAPI),
  getBlockHeaders: getBlockHeaders(dashcoreAPI),
  sendRawTransaction: sendRawTransaction(dashcoreAPI),
  sendRawIxTransaction: sendRawIxTransaction(dashcoreAPI),
  getQuorum: getQuorum(dashcoreAPI),

  // Methods that are using DashDrive
  sendRawTransition: sendRawTransition(dashcoreAPI, dashDriveAPI),
  fetchDapContract: fetchDapContract(dashDriveAPI),
  fetchDapObjects: fetchDapObjects(dashDriveAPI),
  searchUsers: searchUsers(userIndex),
});

const createRegtestCommands = dashcoreAPI => ({
  generate: generate(dashcoreAPI),
});

const createSpvServiceCommands = spvService => ({
  loadBloomFilter: loadBloomFilter(spvService),
  addToBloomFilter: addToBloomFilter(spvService),
  clearBloomFilter: clearBloomFilter(spvService),
  getSpvData: getSpvData(spvService),
  findDataForBlock: findDataForBlock(spvService),
});

/**
  * Starts RPC server
  * @param {number} port - port to listen for incoming RPC connections
  * @param {string} networkType
  * @param {object} spvService
  * @param {object} insightAPI
  * @param {object} dashcoreAPI
  * @param {AbstractDashDriveAdapter} dashDriveAPI - DashDrive api adapter
  * @param {object} userIndex
 */
const start = (port, networkType, spvService, insightAPI, dashcoreAPI, dashDriveAPI, userIndex) => {
  const spvCommands = createSpvServiceCommands(spvService);
  const commands = createCommands(insightAPI, dashcoreAPI, dashDriveAPI, userIndex);
  const areRegtestCommandsEnabled = isRegtest(networkType) || isDevnet(networkType);

  const allCommands = areRegtestCommandsEnabled
    ? Object.assign(commands, spvCommands, createRegtestCommands(dashcoreAPI))
    : Object.assign(commands, spvCommands);

  /*
  Decorate all commands with decorator that will intercept errors and format
  them before passing to user.
  */
  Object.keys(allCommands).forEach((commandName) => {
    allCommands[commandName] = errorHandlerDecorator(allCommands[commandName]);
  });

  const server = jayson.server(allCommands);
  server.http().listen(port);
};

module.exports = {
  createCommands,
  start,
};

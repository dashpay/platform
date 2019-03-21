const jsutil = require('@dashevo/dashcore-lib').util.js;
const preconditionsUtil = require('@dashevo/dashcore-lib').util.preconditions;
const MNDiscovery = require('./MNDiscovery/index');
const rpcClient = require('./RPCClient');
const config = require('./config');

class DAPIClient {
  /**
   * @param options
   * @param {Array<Object>} [options.seeds] - seeds. If no seeds provided
   * default seed will be used.
   * @param {number} [options.port] - default port for connection to the DAPI
   * @param {number} [options.timeout] - timeout for connection to the DAPI
   * @param {number} [options.retries] - num of retries if there is no response from DAPI node
   */
  constructor(options = {}) {
    this.MNDiscovery = new MNDiscovery(options.seeds, options.port);
    this.DAPIPort = options.port || config.Api.port;
    this.timeout = options.timeout || 2000;
    preconditionsUtil.checkArgument(jsutil.isUnsignedInteger(this.timeout),
      'Expect timeout to be an unsigned integer');
    this.retries = options.retries ? options.retries : 3;
    preconditionsUtil.checkArgument(jsutil.isUnsignedInteger(this.retries),
      'Expect retries to be an unsigned integer');
  }

  /**
   * @private
   * @param method
   * @param params
   * @returns {Promise<*>}
   */
  async makeRequestToRandomDAPINode(method, params) {
    this.makeRequest.callCount = 0;
    return this.makeRequestWithRetries(method, params, this.retries);
  }

  async makeRequest(method, params) {
    this.makeRequest.callCount += 1;
    const randomMasternode = await this.MNDiscovery.getRandomMasternode();
    if (!randomMasternode) {
      throw new Error("Can't connect to DAPI: Masternode list is empty! Please try again later");
    }
    return rpcClient.request({
      host: randomMasternode.service.split(':')[0],
      port: this.DAPIPort,
    }, method, params, { timeout: this.timeout });
  }

  async makeRequestWithRetries(method, params, retriesCount = 0) {
    try {
      return await this.makeRequest(method, params);
    } catch (err) {
      if (err.code !== 'ECONNABORTED' && err.code !== 'ECONNREFUSED') {
        throw new Error(`DAPI RPC error: ${method}: ${err}`);
      }
      if (retriesCount > 0) {
        return this.makeRequestWithRetries(method, params, retriesCount - 1);
      }
      throw new Error('max retries to connect to DAPI node reached');
    }
  }

  /* Layer 1 commands */

  /**
   * Estimate fee
   * @param {number} numberOfBlocksToWait
   * @return {Promise<number>} - duffs per byte
   */
  estimateFee(numberOfBlocksToWait) { return this.makeRequestToRandomDAPINode('estimateFee', { blocks: numberOfBlocksToWait }); }

  /**
   * ONLY FOR TESTING PURPOSES WITH REGTEST. WILL NOT WORK ON TESTNET/LIVENET.
   * @param {number} amount - Number of blocks to generate
   * @returns {Promise<string[]>} - block hashes
   */
  generate(amount) { return this.makeRequestToRandomDAPINode('generate', { amount }); }

  /**
   * Returns a summary (balance, txs) for a given address
   * @param {string} address
   * @returns {Promise<Object>} - an object with basic address info
   */
  getAddressSummary(address) { return this.makeRequestToRandomDAPINode('getAddressSummary', { address }); }

  /**
   * @param {string} address
   * @return {Promise<number>}
   */
  getAddressTotalSent(address) { return this.makeRequestToRandomDAPINode('getAddressTotalSent', { address }); }

  /**
   * @param {string} address
   * @return {Promise<number>}
   */
  getAddressUnconfirmedBalance(address) { return this.makeRequestToRandomDAPINode('getAddressUnconfirmedBalance', { address }); }

  /**
   * @param {string} address
   * @return {Promise<number>}
   */
  getAddressTotalReceived(address) { return this.makeRequestToRandomDAPINode('getAddressTotalReceived', { address }); }

  /**
   * Returns balance for a given address
   * @param {string} address
   * @returns {Promise<number>} - address balance
   */
  getBalance(address) { return this.makeRequestToRandomDAPINode('getBalance', { address }); }

  /**
   * Returns block hash of chaintip
   * @returns {Promise<string>}
   */
  getBestBlockHash() { return this.makeRequestToRandomDAPINode('getBestBlockHash', {}); }

  /**
   * Returns best block height
   * @returns {Promise<number>}
   */
  getBestBlockHeight() { return this.makeRequestToRandomDAPINode('getBestBlockHeight', {}); }

  /**
   * Returns block hash for the given height
   * @param {number} height
   * @returns {Promise<string>} - block hash
   */
  getBlockHash(height) { return this.makeRequestToRandomDAPINode('getBlockHash', { height }); }

  /**
   * Returns block header by hash
   * @param {string} blockHash
   * @returns {Promise<[objects]>} - array of header objects
   */
  getBlockHeader(blockHash) { return this.makeRequestToRandomDAPINode('getBlockHeader', { blockHash }); }

  /**
   * Returns block headers from [offset] with length [limit], where limit is <= 25
   * @param {number} offset
   * @param {number} limit
   * @returns {Promise<[objects]>} - array of header objects
   */
  getBlockHeaders(offset, limit) { return this.makeRequestToRandomDAPINode('getBlockHeaders', { offset, limit }); }

  // TODO: Do we really need it this way?
  /**
   * Get block summaries for the day
   * @param {string} blockDate string in format 'YYYY-MM-DD'
   * @param limit
   * @return {Promise<object>}
   */
  getBlocks(blockDate, limit) { return this.makeRequestToRandomDAPINode('getBlocks', { blockDate, limit }); }

  /**
   * @return {Promise<object>}
   */
  getHistoricBlockchainDataSyncStatus() { return this.makeRequestToRandomDAPINode('getHistoricBlockchainDataSyncStatus', {}); }

  /**
   * Returns mempool usage info
   * @returns {Promise<object>}
   */
  getMempoolInfo() { return this.makeRequestToRandomDAPINode('getMempoolInfo', {}); }

  /**
   * Get deterministic masternodelist diff
   * @param {string} baseBlockHash - hash or height of start block
   * @param {string} blockHash - hash or height of end block
   * @return {Promise<object>}
   */
  getMnListDiff(baseBlockHash, blockHash) { return this.makeRequestToRandomDAPINode('getMnListDiff', { baseBlockHash, blockHash }); }

  /**
   * @param {string} blockHash
   * @return {Promise<object>}
   */
  getRawBlock(blockHash) { return this.makeRequestToRandomDAPINode('getRawBlock', { blockHash }); }

  /**
   * @param {string} txid - transaction hash
   * @return {Promise<object>}
   */
  getTransaction(txid) { return this.getTransactionById(txid); }

  /**
   * Returns Transactions for a given address or multiple addresses
   * @param address
   * @param {string|string[]} address or array of addresses
   * @param {number} from - start of range in the ordered list of latest UTXO (optional)
   * @param {number} to - end of range in the ordered list of latest UTXO (optional)
   * @param {number} fromHeight - which height to start from (optional, overriding from/to)
   * @param {number} toHeight - on which height to end (optional, overriding from/to)
   * @returns {Promise<object>} - Object with pagination info and array of unspent outputs
   */
  getTransactionsByAddress(address, from, to, fromHeight, toHeight) {
    return this.makeRequestToRandomDAPINode('getTransactionsByAddress',
      {
        address, from, to, fromHeight, toHeight,
      });
  }

  /**
   * @param {string} txid - transaction hash
   * @return {Promise<object>}
   */
  getTransactionById(txid) { return this.makeRequestToRandomDAPINode('getTransactionById', { txid }); }

  /**
   * Returns UTXO for a given address or multiple addresses (max result 1000)
   * @param {string|string[]} address or array of addresses
   * @param {number} from - start of range in the ordered list of latest UTXO (optional)
   * @param {number} to - end of range in the ordered list of latest UTXO (optional)
   * @param {number} fromHeight - which height to start from (optional, overriding from/to)
   * @param {number} toHeight - on which height to end (optional, overriding from/to)
   * @returns {Promise<object>} - Object with pagination info and array of unspent outputs
   */
  getUTXO(address, from, to, fromHeight, toHeight) {
    return this.makeRequestToRandomDAPINode('getUTXO',
      {
        address, from, to, fromHeight, toHeight,
      });
  }

  /**
   * @param {string} rawIxTransaction - hex-serialized instasend transaction
   * @return {Promise<string>} - transaction id
   */
  sendRawIxTransaction(rawIxTransaction) { return this.makeRequestToRandomDAPINode('sendRawIxTransaction', { rawIxTransaction }); }

  /**
   * Sends serialized transaction to the network
   * @param {string} rawTransaction - hex string representing serialized transaction
   * @returns {Promise<string>} - transaction id
   */
  sendRawTransaction(rawTransaction) { return this.makeRequestToRandomDAPINode('sendRawTransaction', { rawTransaction }); }

  /* Layer 2 commands */

  fetchDapContract(contractId) { return this.makeRequestToRandomDAPINode('fetchDapContract', { contractId }); }

  /**
   * Fetch DAP Objects from DashDrive State View
   * @param {string} contractId
   * @param {string} type - Dap objects type to fetch
   * @param options
   * @param {Object} options.where - Mongo-like query
   * @param {Object} options.orderBy - Mongo-like sort field
   * @param {number} options.limit - how many objects to fetch
   * @param {number} options.startAt - number of objects to skip
   * @param {number} options.startAfter - exclusive skip
   * @return {Promise<Object[]>}
   */
  fetchDapObjects(contractId, type, options) { return this.makeRequestToRandomDAPINode('fetchDapObjects', { contractId, type, options }); }

  /**
   * Returns blockchain user by its username or regtx id
   * @param {string} userId - user reg tx id
   * @returns {Promise<Object>} - blockchain user
   */
  getUserById(userId) { return this.makeRequestToRandomDAPINode('getUser', { userId }); }

  /**
   * Returns blockchain user by its username or regtx id
   * @param {string} username
   * @returns {Promise<Object>} - blockchain user
   */
  getUserByName(username) { return this.makeRequestToRandomDAPINode('getUser', { username }); }

  /**
   * Sends serialized state transition header and data packet
   * @param {string} rawStateTransition - hex string representing state transition header
   * @param {string} rawSTPacket - hex string representing state transition data
   * @returns {Promise<string>} - header id
   */
  sendRawTransition(rawStateTransition, rawSTPacket) {
    return this.makeRequestToRandomDAPINode('sendRawTransition', { rawStateTransition, rawSTPacket });
  }

  // Here go methods that used in VMN. Most of this methods will work only in regtest mode
  searchUsers(pattern, limit = 10, offset = 0) { return this.makeRequestToRandomDAPINode('searchUsers', { pattern, limit, offset }); }


  // Temp methods for SPV testing/POC
  // In future SPV will choose a specific node and stick with
  // the node for as long as possible for SPV interaction (to prevent dapi chain rescans)
  loadBloomFilter(filter) { return this.makeRequestToRandomDAPINode('loadBloomFilter', { filter }); }

  addToBloomFilter(originalFilter, element) { return this.makeRequestToRandomDAPINode('addToBloomFilter', { originalFilter, element }); }

  clearBloomFilter(filter) { return this.makeRequestToRandomDAPINode('clearBloomFilter', { filter }); }

  getSpvData(filter) { return this.makeRequestToRandomDAPINode('getSpvData', { filter }); }
}

module.exports = DAPIClient;

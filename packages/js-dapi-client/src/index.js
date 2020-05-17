const jsutil = require('@dashevo/dashcore-lib').util.js;
const preconditionsUtil = require('@dashevo/dashcore-lib').util.preconditions;
const cbor = require('cbor');

const {
  TransactionsWithProofsRequest,
  BloomFilter: BloomFilterMessage,
  ApplyStateTransitionRequest,
  GetIdentityRequest,
  GetDataContractRequest,
  GetDocumentsRequest,
  GetBlockRequest,
  GetStatusRequest,
  GetTransactionRequest,
  SendTransactionRequest,
} = require('@dashevo/dapi-grpc');

const MNDiscovery = require('./MNDiscovery/index');
const TransportManager = require('./transport/TransportManager');
const config = require('./config');
const { responseErrorCodes } = require('./constants');

class DAPIClient {
  /**
   * @param options
   * @param {Array<Object>} [options.seeds] - seeds. If no seeds provided
   * default seed will be used.
   * @param {number} [options.port=3000] - default port for connection to the DAPI
   * @param {number} [options.nativeGrpcPort=3010] - Native GRPC port for connection to the DAPI
   * @param {number} [options.timeout=2000] - timeout for connection to the DAPI
   * @param {number} [options.retries=3] - num of retries if there is no response from DAPI node
   */
  constructor(options = {}) {
    this.MNDiscovery = new MNDiscovery(options.seeds, options.port);
    this.DAPIPort = options.port || config.Api.port;
    this.nativeGrpcPort = options.nativeGrpcPort || config.grpc.nativePort;

    this.timeout = options.timeout || 2000;
    preconditionsUtil.checkArgument(jsutil.isUnsignedInteger(this.timeout),
      'Expect timeout to be an unsigned integer');

    this.retries = options.retries ? options.retries : 3;
    preconditionsUtil.checkArgument(jsutil.isUnsignedInteger(this.retries),
      'Expect retries to be an unsigned integer');

    this.transportManager = new TransportManager(
      this.MNDiscovery, this.DAPIPort, this.nativeGrpcPort,
    );
  }

  /* Layer 1 commands */
  /**
   * ONLY FOR TESTING PURPOSES WITH REGTEST. WILL NOT WORK ON TESTNET/LIVENET.
   * @param {number} blocksNumber - Number of blocks to generate
   * @param {string} address - The address that will receive the newly generated Dash
   * @returns {Promise<string[]>} - block hashes
   */
  generateToAddress(blocksNumber, address) {
    return this.transportManager.get(TransportManager.JSON_RPC)
      .makeRequest(
        'generateToAddress', { blocksNumber, address },
        { retriesCount: this.retries, client: { timeout: this.timeout } },
      );
  }

  /**
   * Returns block hash of chaintip
   * @returns {Promise<string>}
   */
  getBestBlockHash() {
    return this.transportManager.get(TransportManager.JSON_RPC)
      .makeRequest(
        'getBestBlockHash', {},
        { retriesCount: this.retries, client: { timeout: this.timeout } },
      );
  }

  /**
   * Returns block hash for the given height
   * @param {number} height
   * @returns {Promise<string>} - block hash
   */
  getBlockHash(height) {
    return this.transportManager.get(TransportManager.JSON_RPC)
      .makeRequest(
        'getBlockHash', { height },
        { retriesCount: this.retries, client: { timeout: this.timeout } },
      );
  }

  /**
   * Get deterministic masternodelist diff
   * @param {string} baseBlockHash - hash or height of start block
   * @param {string} blockHash - hash or height of end block
   * @return {Promise<object>}
   */
  getMnListDiff(baseBlockHash, blockHash) {
    return this.transportManager.get(TransportManager.JSON_RPC)
      .makeRequest(
        'getMnListDiff', { baseBlockHash, blockHash },
        { retriesCount: this.retries, client: { timeout: this.timeout } },
      );
  }

  /**
   * Returns a summary (balance, txs) for a given address
   * @param {string|string[]} address or array of addresses
   * @param {boolean} [noTxList=false] - true if a list of all txs should NOT be included in result
   * @param {number} [from] - start of range for the tx to be included in the tx list
   * @param {number} [to] - end of range for the tx to be included in the tx list
   * @param {number} [fromHeight] - which height to start from (optional, overriding from/to)
   * @param {number} [toHeight] - on which height to end (optional, overriding from/to)
   * @returns {Promise<Object>} - an object with basic address info
   */
  getAddressSummary(address, noTxList, from, to, fromHeight, toHeight) {
    return this.transportManager.get(TransportManager.JSON_RPC).makeRequest(
      'getAddressSummary',
      {
        address, noTxList, from, to, fromHeight, toHeight,
      },
      { retriesCount: this.retries, client: { timeout: this.timeout } },
    );
  }

  /**
   * Get block by height
   *
   * @param {number} height
   * @return {Promise<null|Buffer>}
   */
  async getBlockByHeight(height) {
    const getBlockRequest = new GetBlockRequest();
    getBlockRequest.setHeight(height);

    let response;
    try {
      response = await this.transportManager.get(TransportManager.GRPC_CORE)
        .makeRequest(
          'getBlock', getBlockRequest,
          { retriesCount: this.retries, client: { timeout: this.timeout } },
        );
    } catch (e) {
      if (e.code === responseErrorCodes.NOT_FOUND) {
        return null;
      }

      throw e;
    }

    const blockBinaryArray = response.getBlock();

    return Buffer.from(blockBinaryArray);
  }

  /**
   * Get block by hash
   *
   * @param {string} hash
   * @return {Promise<null|Buffer>}
   */
  async getBlockByHash(hash) {
    const getBlockRequest = new GetBlockRequest();
    getBlockRequest.setHash(hash);

    let response;
    try {
      response = await this.transportManager.get(TransportManager.GRPC_CORE)
        .makeRequest(
          'getBlock', getBlockRequest,
          { retriesCount: this.retries, client: { timeout: this.timeout } },
        );
    } catch (e) {
      if (e.code === responseErrorCodes.NOT_FOUND) {
        return null;
      }

      throw e;
    }

    const blockBinaryArray = response.getBlock();

    return Buffer.from(blockBinaryArray);
  }

  /**
   * Get Core chain status
   *
   * @return {Promise<Object>}
   */
  async getStatus() {
    const getStatusRequest = new GetStatusRequest();

    const response = await this.transportManager.get(TransportManager.GRPC_CORE)
      .makeRequest(
        'getStatus', getStatusRequest,
        { retriesCount: this.retries, client: { timeout: this.timeout } },
      );

    return response.toObject();
  }

  /**
   * Get Transaction by ID
   *
   * @param {string} id
   * @return {Promise<null|Buffer>}
   */
  async getTransaction(id) {
    const getTransactionRequest = new GetTransactionRequest();
    getTransactionRequest.setId(id);

    let response;
    try {
      response = await this.transportManager.get(TransportManager.GRPC_CORE)
        .makeRequest(
          'getTransaction', getTransactionRequest,
          { retriesCount: this.retries, client: { timeout: this.timeout } },
        );
    } catch (e) {
      if (e.code === responseErrorCodes.NOT_FOUND) {
        return null;
      }

      throw e;
    }

    const transactionBinaryArray = response.getTransaction();

    let transaction = null;
    if (transactionBinaryArray) {
      transaction = Buffer.from(transactionBinaryArray);
    }

    return transaction;
  }

  /**
   * Send Transaction
   *
   * @param {Buffer} transaction
   * @param {Object} [options]
   * @param {Object} [options.allowHighFees=false]
   * @param {Object} [options.bypassLimits=false]
   * @return {string}
   */
  async sendTransaction(transaction, options = {}) {
    const sendTransactionRequest = new SendTransactionRequest();
    sendTransactionRequest.setTransaction(transaction);
    sendTransactionRequest.setAllowHighFees(options.allowHighFees || false);
    sendTransactionRequest.setBypassLimits(options.bypassLimits || false);

    const response = await this.transportManager.get(TransportManager.GRPC_CORE)
      .makeRequest(
        'sendTransaction', sendTransactionRequest,
        { retriesCount: this.retries, client: { timeout: this.timeout } },
      );

    return response.getTransactionId();
  }

  /**
   * Returns UTXO for a given address or multiple addresses (max result 1000)
   * @param {string|string[]} address or array of addresses
   * @param {number} [from] - start of range in the ordered list of latest UTXO (optional)
   * @param {number} [to] - end of range in the ordered list of latest UTXO (optional)
   * @param {number} [fromHeight] - which height to start from (optional, overriding from/to)
   * @param {number} [toHeight] - on which height to end (optional, overriding from/to)
   * @returns {Promise<object>} - Object with pagination info and array of unspent outputs
   */
  getUTXO(address, from, to, fromHeight, toHeight) {
    return this.transportManager.get(TransportManager.JSON_RPC).makeRequest(
      'getUTXO',
      {
        address, from, to, fromHeight, toHeight,
      },
      { retriesCount: this.retries, client: { timeout: this.timeout } },
    );
  }

  /* gRPC methods */

  /* txFilterStream methods */
  /**
   * @param {Object} bloomFilter
   * @param {Uint8Array|Array} bloomFilter.vData - The filter itself is simply a bit
   * field of arbitrary byte-aligned size. The maximum size is 36,000 bytes.
   * @param {number} bloomFilter.nHashFuncs - The number of hash functions to use in this filter.
   * The maximum value allowed in this field is 50.
   * @param {number} bloomFilter.nTweak - A random value to add to the seed value in the
   * hash function used by the bloom filter.
   * @param {number} bloomFilter.nFlags - A set of flags that control how matched items
   * are added to the filter.
   * @param {Object} [options]
   * @param {string} [options.fromBlockHash] - Specifies block hash to start syncing from
   * @param {number} [options.fromBlockHeight] - Specifies block height to start syncing from
   * @param {number} [options.count=0] - Number of blocks to sync,
   * if set to 0 syncing is continuously sends new data as well
   * @returns {
   *    Promise<EventEmitter>|!grpc.web.ClientReadableStream<!TransactionsWithProofsResponse>
   * }
   */
  async subscribeToTransactionsWithProofs(bloomFilter, options = { count: 0 }) {
    const bloomFilterMessage = new BloomFilterMessage();

    let { vData } = bloomFilter;
    if (Array.isArray(vData)) {
      vData = new Uint8Array(vData);
    }

    bloomFilterMessage.setVData(vData);
    bloomFilterMessage.setNHashFuncs(bloomFilter.nHashFuncs);
    bloomFilterMessage.setNTweak(bloomFilter.nTweak);
    bloomFilterMessage.setNFlags(bloomFilter.nFlags);

    const request = new TransactionsWithProofsRequest();
    request.setBloomFilter(bloomFilterMessage);

    if (options.fromBlockHeight) {
      request.setFromBlockHeight(options.fromBlockHeight);
    }

    if (options.fromBlockHash) {
      request.setFromBlockHash(
        Buffer.from(options.fromBlockHash, 'hex'),
      );
    }

    request.setCount(options.count);

    return this.transportManager.get(TransportManager.GRPC_TX)
      .makeRequest(
        'subscribeToTransactionsWithProofs', request,
        { retriesCount: this.retries, client: { timeout: this.timeout } },
      );
  }

  /* Platform gRPC methods */

  /**
   * Send State Transition to machine
   *
   * @param {AbstractStateTransition} stateTransition
   * @returns {Promise<!ApplyStateTransitionResponse>}
   */
  async applyStateTransition(stateTransition) {
    const applyStateTransitionRequest = new ApplyStateTransitionRequest();
    applyStateTransitionRequest.setStateTransition(stateTransition.serialize());

    return this.transportManager.get(TransportManager.GRPC_PLATFORM)
      .makeRequest(
        'applyStateTransition', applyStateTransitionRequest,
        { retriesCount: this.retries, client: { timeout: this.timeout } },
      );
  }

  /**
   * Fetch the identity by id
   * @param {string} id
   * @returns {Promise<!Buffer|null>}
   */
  async getIdentity(id) {
    const getIdentityRequest = new GetIdentityRequest();
    getIdentityRequest.setId(id);

    let getIdentityResponse;
    try {
      getIdentityResponse = await this.transportManager.get(TransportManager.GRPC_PLATFORM)
        .makeRequest(
          'getIdentity', getIdentityRequest,
          { retriesCount: this.retries, client: { timeout: this.timeout } },
        );
    } catch (e) {
      if (e.code === responseErrorCodes.NOT_FOUND) {
        return null;
      }

      throw e;
    }

    const serializedIdentityBinaryArray = getIdentityResponse.getIdentity();
    let identity = null;

    if (serializedIdentityBinaryArray) {
      identity = Buffer.from(serializedIdentityBinaryArray);
    }

    return identity;
  }

  /**
   * Fetch Data Contract by id
   * @param {string} contractId
   * @returns {Promise<Buffer>}
   */
  async getDataContract(contractId) {
    const getDataContractRequest = new GetDataContractRequest();

    getDataContractRequest.setId(contractId);

    let getDataContractResponse;
    try {
      getDataContractResponse = await this.transportManager.get(TransportManager.GRPC_PLATFORM)
        .makeRequest(
          'getDataContract', getDataContractRequest,
          { retriesCount: this.retries, client: { timeout: this.timeout } },
        );
    } catch (e) {
      if (e.code === responseErrorCodes.NOT_FOUND) {
        return null;
      }

      throw e;
    }

    const serializedDataContractBinaryArray = getDataContractResponse.getDataContract();

    let dataContract = null;

    if (serializedDataContractBinaryArray) {
      dataContract = Buffer.from(serializedDataContractBinaryArray);
    }

    return dataContract;
  }

  /**
   * Fetch Documents from Drive
   * @param {string} contractId
   * @param {string} type - Dap objects type to fetch
   * @param options
   * @param {Object} options.where - Mongo-like query
   * @param {Object} options.orderBy - Mongo-like sort field
   * @param {number} options.limit - how many objects to fetch
   * @param {number} options.startAt - number of objects to skip
   * @param {number} options.startAfter - exclusive skip
   * @return {Promise<Buffer[]>}
   */
  async getDocuments(contractId, type, options) {
    const {
      where,
      orderBy,
      limit,
      startAt,
      startAfter,
    } = options;

    let whereSerialized;
    if (where) {
      whereSerialized = cbor.encode(where);
    }

    let orderBySerialized;
    if (orderBy) {
      orderBySerialized = cbor.encode(orderBy);
    }

    const getDocumentsRequest = new GetDocumentsRequest();
    getDocumentsRequest.setDataContractId(contractId);
    getDocumentsRequest.setDocumentType(type);
    getDocumentsRequest.setWhere(whereSerialized);
    getDocumentsRequest.setOrderBy(orderBySerialized);
    getDocumentsRequest.setLimit(limit);
    getDocumentsRequest.setStartAfter(startAfter);
    getDocumentsRequest.setStartAt(startAt);

    const getDocumentsResponse = await this.transportManager.get(TransportManager.GRPC_PLATFORM)
      .makeRequest(
        'getDocuments', getDocumentsRequest,
        { retriesCount: this.retries, client: { timeout: this.timeout } },
      );

    return getDocumentsResponse.getDocumentsList().map((document) => Buffer.from(document));
  }
}

module.exports = DAPIClient;

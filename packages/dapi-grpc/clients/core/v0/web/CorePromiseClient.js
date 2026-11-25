// Inline promisify shim — avoids requiring Node's `util` module so this file
// can be bundled for browsers without a polyfill. If the codegen template
// is regenerated, restore this shim.
function promisify(fn) {
  return (...args) => new Promise((resolve, reject) => {
    fn(...args, (err, result) => (err ? reject(err) : resolve(result)));
  });
}
const rewireStream = require('../../../../lib/utils/rewireStream');

const { CoreClient } = require('./core_pb_service');

class CorePromiseClient {
  /**
   * @param {string} hostname
   * @param {?Object} credentials
   * @param {?Object} options
   */
  constructor(hostname, credentials , options = {}) {
    this.client = new CoreClient(hostname, options)
  }

  /**
   * @param {!GetBlockchainStatusRequest} getBlockchainStatusRequest
   * @param {?Object<string, string>} metadata
   * @return {Promise<!GetBlockchainStatusResponse>}
   */
  getBlockchainStatus(getBlockchainStatusRequest, metadata = {}) {
    return promisify(
      this.client.getBlockchainStatus.bind(this.client),
    )(
      getBlockchainStatusRequest,
      metadata,
    );
  }

  /**
   * @param {!GetMasternodeStatusRequest} getMasternodeStatusRequest
   * @param {?Object<string, string>} metadata
   * @return {Promise<!GetMasternodeStatusResponse>}
   */
  getMasternodeStatus(getMasternodeStatusRequest, metadata = {}) {
    return promisify(
      this.client.getMasternodeStatus.bind(this.client),
    )(
      getMasternodeStatusRequest,
      metadata,
    );
  }

  /**
   * @param {!GetBlockRequest} getBlockRequest
   * @param {?Object<string, string>} metadata
   * @return {Promise<!GetBlockResponse>}
   */
  getBlock(getBlockRequest, metadata = {}) {
    return promisify(
      this.client.getBlock.bind(this.client),
    )(
      getBlockRequest,
      metadata,
    );
  }

  /**
   * @param {!GetBestBlockHeightRequest} getBestBlockHeightRequest
   * @param {?Object<string, string>} metadata
   * @return {Promise<!GetBestBlockHeightResponse>}
   */
  getBestBlockHeight(getBestBlockHeightRequest, metadata = {}) {
    return promisify(
      this.client.getBestBlockHeight.bind(this.client),
    )(
      getBestBlockHeightRequest,
      metadata,
    );
  }

  /**
   * @param {!BroadcastTransactionRequest} broadcastTransactionRequest
   * @param {?Object<string, string>} metadata
   * @return {Promise<!BroadcastTransactionResponse>}
   */
  broadcastTransaction(broadcastTransactionRequest, metadata = {}) {
    return promisify(
      this.client.broadcastTransaction.bind(this.client),
    )(
      broadcastTransactionRequest,
      metadata,
    );
  }

  /**
   * @param {!GetTransactionRequest} getTransactionRequest
   * @param {?Object<string, string>} metadata
   * @return {Promise<!GetTransactionResponse>}
   */
  getTransaction(getTransactionRequest, metadata = {}) {
    return promisify(
      this.client.getTransaction.bind(this.client),
    )(
      getTransactionRequest,
      metadata,
    );
  }

  /**
   * @param {!GetEstimatedTransactionFeeRequest} getEstimatedTransactionFeeRequest
   * @param {?Object<string, string>} metadata
   * @returns {Promise<!GetEstimatedTransactionFeeResponse>}
   */
  getEstimatedTransactionFee(getEstimatedTransactionFeeRequest, metadata = {}) {
    return promisify(
      this.client.getEstimatedTransactionFee.bind(this.client),
    )(
      getEstimatedTransactionFeeRequest,
      metadata,
    );
  }

  /**
   * @param {!BlockHeadersWithChainLocksRequest} blockHeadersWithChainLocksRequest
   * @param {?Object<string, string>} metadata
   * @return {!grpc.web.ClientReadableStream<!BlockHeadersWithChainLocksResponse>|undefined}
   */
  subscribeToBlockHeadersWithChainLocks(
    blockHeadersWithChainLocksRequest,
    metadata = {},
  ) {
    const stream = this.client.subscribeToBlockHeadersWithChainLocks(
      blockHeadersWithChainLocksRequest,
      metadata,
    )

    rewireStream(stream);

    return stream;
  }

  /**
   * @param {TransactionsWithProofsRequest} transactionsWithProofsRequest The request proto
   * @param {?Object<string, string>} metadata User defined call metadata
   * @return {!grpc.web.ClientReadableStream<!TransactionsWithProofsResponse>|undefined}
   */
  subscribeToTransactionsWithProofs(transactionsWithProofsRequest, metadata = {}) {
    const stream = this.client.subscribeToTransactionsWithProofs(
      transactionsWithProofsRequest,
      metadata
    )

    rewireStream(stream);
    return stream;
  }

  /**
   * @param {MasternodeListRequest} masternodeListRequest The request proto
   * @param {?Object<string, string>} metadata User defined call metadata
   * @return {!grpc.web.ClientReadableStream<!MasternodeListResponse>|undefined}
   */
  subscribeToMasternodeList(masternodeListRequest, metadata = {}) {
    const stream = this.client.subscribeToMasternodeList(
      masternodeListRequest,
      metadata
    )

    rewireStream(stream);

    return stream;
  }
}

module.exports = CorePromiseClient;

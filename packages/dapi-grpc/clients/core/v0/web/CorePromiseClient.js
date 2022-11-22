const { promisify } = require('util');
const GrpcError = require('@dashevo/grpc-common/lib/server/error/GrpcError');

const { CoreClient } = require('./core_pb_service');

/**
 * Function rewires @imporbable-eng/grpc-web stream
 * to comply with the EventEmitter interface
 * @param stream
 * @return {!grpc.web.ClientReadableStream}
 */
const rewireStream = (stream) => {
  const defaultOnFunction = stream.on.bind(stream);

  // TODO: remove after SPV refactoring
  // This is only needed for hacks applied by legacy TransactionSyncStreamWorker
  // onError events
  stream.f = [];
  // onEnd events
  stream.c = []

  // Rewire default on function to comply with EventEmitter interface
  stream.on = ((type, handler) => {
    if (type === 'end') {
      stream.c.push(handler);
      return defaultOnFunction(type, handler);
    } else if (type === 'error') { // Handle `error` event using `end` event
      stream.f.push(handler);
      return stream.on('end', (payload) => {
        if (payload) {
          const { code, details, metadata } = payload;
          if (code !== 0) {
            handler(new GrpcError(code, details, metadata));
          }
        }
      });
    } else {
      // `data` event could be processed normally
      return defaultOnFunction(type, handler);
    }
  });

  // Assign an empty function to `once` method
  // because @imporbable-eng/grpc-web doesn't expose it
  // stream cancellation detaches all handlers internally
  stream.removeListener = () => {}
}

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
   * @param {!GetStatusRequest} getStatusRequest
   * @param {?Object<string, string>} metadata
   * @return {Promise<!GetStatusResponse>}
   */
  getStatus(getStatusRequest, metadata = {}) {
    return promisify(
      this.client.getStatus.bind(this.client),
    )(
      getStatusRequest,
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
}

module.exports = CorePromiseClient;

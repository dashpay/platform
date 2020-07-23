const {
  CorePromiseClient,
  BroadcastTransactionRequest,
} = require('@dashevo/dapi-grpc');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {broadcastTransaction}
 */
function broadcastTransactionFactory(grpcTransport) {
  /**
   * Broadcast Transaction
   *
   * @typedef {broadcastTransaction}
   * @param {Buffer} transaction
   * @param {DAPIClientOptions & BroadcastTransactionOptions} [options]
   * @returns {string}
   */
  async function broadcastTransaction(transaction, options = {}) {
    const broadcastTransactionRequest = new BroadcastTransactionRequest();
    broadcastTransactionRequest.setTransaction(transaction);
    broadcastTransactionRequest.setAllowHighFees(options.allowHighFees || false);
    broadcastTransactionRequest.setBypassLimits(options.bypassLimits || false);

    const response = await grpcTransport.request(
      CorePromiseClient,
      'broadcastTransaction',
      broadcastTransactionRequest,
      options,
    );

    return response.getTransactionId();
  }

  return broadcastTransaction;
}

/**
 * @typedef {object} BroadcastTransactionOptions
 * @property {boolean} [allowHighFees=false]
 * @property {boolean} [bypassLimits=false]
 */

module.exports = broadcastTransactionFactory;

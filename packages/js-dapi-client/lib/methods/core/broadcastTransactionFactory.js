const {
  CorePromiseClient,
  SendTransactionRequest,
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
    const sendTransactionRequest = new SendTransactionRequest();
    sendTransactionRequest.setTransaction(transaction);
    sendTransactionRequest.setAllowHighFees(options.allowHighFees || false);
    sendTransactionRequest.setBypassLimits(options.bypassLimits || false);

    const response = await grpcTransport.request(
      CorePromiseClient,
      'sendTransaction',
      sendTransactionRequest,
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

const cbor = require('cbor');

const {
  server: {
    error: {
      InternalGrpcError,
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  UpdateStateTransitionResponse,
} = require('@dashevo/dapi-grpc');

/**
 *
 * @param {jaysonClient} rpcClient
 * @returns {updateStateHandler}
 */
function updateStateHandlerFactory(rpcClient) {
  /**
   * @typedef updateStateHandler
   * @param {Object} call
   */
  async function updateStateHandler(call) {
    const { request } = call;
    const header = request.getHeader();
    const packet = request.getPacket();

    if (!header) {
      throw new InvalidArgumentGrpcError('header is not specified');
    }

    if (!packet) {
      throw new InvalidArgumentGrpcError('packet is not specified');
    }

    const st = {
      header: Buffer.from(header).toString('hex'),
      packet: Buffer.from(packet),
    };

    const tx = cbor.encodeCanonical(st).toString('base64');

    let result;
    let error;
    try {
      // @TODO check for timeout
      ({ result, error } = await rpcClient.request('broadcast_tx_commit', { tx }));
    } catch (e) {
      throw new InternalGrpcError(e);
    }

    if (error) {
      throw new InternalGrpcError(error);
    }

    const { check_tx: checkTx, deliver_tx: deliverTx } = result;

    if (checkTx.code > 0) {
      const { error: { message, data } } = JSON.parse(checkTx.log);

      throw new InvalidArgumentGrpcError(message, data);
    }

    if (deliverTx.code > 0) {
      const { error: { message, data } } = JSON.parse(checkTx.log);

      throw new InvalidArgumentGrpcError(message, data);
    }

    return new UpdateStateTransitionResponse();
  }

  return updateStateHandler;
}

module.exports = updateStateHandlerFactory;

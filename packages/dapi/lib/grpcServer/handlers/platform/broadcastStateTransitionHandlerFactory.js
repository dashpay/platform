const {
  server: {
    error: {
      InvalidArgumentGrpcError,
      ResourceExhaustedGrpcError,
      UnavailableGrpcError,
      AlreadyExistsGrpcError,
      InternalGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    BroadcastStateTransitionResponse,
  },
} = require('@dashevo/dapi-grpc');

const crypto = require('crypto');

const logger = require('../../../logger');

/**
 * @param {jaysonClient} rpcClient
 * @param {createGrpcErrorFromDriveResponse} createGrpcErrorFromDriveResponse
 * @param {requestTenderRpc} requestTenderRpc
 *
 * @returns {broadcastStateTransitionHandler}
 */
function broadcastStateTransitionHandlerFactory(
  rpcClient,
  createGrpcErrorFromDriveResponse,
  requestTenderRpc,
) {
  /**
   * @typedef broadcastStateTransitionHandler
   *
   * @param {Object} call
   *
   * @return {Promise<BroadcastStateTransitionResponse>}
   */
  async function broadcastStateTransitionHandler(call) {
    const { request } = call;
    const stByteArray = request.getStateTransition();

    if (!stByteArray) {
      throw new InvalidArgumentGrpcError('State Transition is not specified');
    }

    const stBytes = Buffer.from(stByteArray);

    const tx = stBytes
      .toString('base64');

    let response;

    try {
      response = await rpcClient.request('broadcast_tx', { tx });
    } catch (e) {
      if (e.code === 'ECONNRESET' || e.message === 'socket hang up') {
        throw new UnavailableGrpcError('Tenderdash is not available');
      }

      e.message = `Failed broadcasting state transition: ${e.message}`;

      throw e;
    }

    const { result, error: jsonRpcError } = response;

    if (jsonRpcError) {
      if (typeof jsonRpcError.data === 'string') {
        if (jsonRpcError.data === 'tx already exists in cache') {
          // We need to figure out and report to user why the ST cached
          const stHash = crypto.createHash('sha256')
            .update(stBytes)
            .digest();

          // Throw an already exist in mempool error if the ST in mempool
          let unconfirmedTxResponse;
          try {
            unconfirmedTxResponse = await requestTenderRpc(
              'unconfirmed_tx',
              { hash: `0x${stHash.toString('hex')}` },
            );
          } catch (e) {
            if (typeof e.data !== 'string' || !e.data.includes('not found')) {
              throw e;
            }
          }

          if (unconfirmedTxResponse?.tx) {
            throw new AlreadyExistsGrpcError('state transition already in mempool');
          }

          // Throw an already exist in chain error if the ST is committed
          let txResponse;
          try {
            txResponse = await requestTenderRpc('tx', { hash: stHash.toString('base64') });
          } catch (e) {
            if (typeof e.data !== 'string' || !e.data.includes('not found')) {
              throw e;
            }
          }

          if (txResponse?.tx_result) {
            throw new AlreadyExistsGrpcError('state transition already in chain');
          }

          // If the ST not in mempool and not in the state but still in the cache
          // it means it was invalidated by CheckTx so we run CheckTx again to provide
          // the validation error
          const checkTxResponse = await requestTenderRpc('check_tx', { tx });

          if (checkTxResponse?.code !== 0) {
            // Return validation error
            throw await createGrpcErrorFromDriveResponse(
              checkTxResponse.code,
              checkTxResponse.info,
            );
          } else {
            // CheckTx passes for the ST, it means we have a bug in Drive so ST is passing check
            // tx and then removed from the block. The removal from the block doesn't remove ST
            // from the cache because it's happening only one proposer and other nodes do not know
            // that this ST was processed and keep it in the cache
            // The best what we can do is to return an internal error and and log the transaction
            logger.warn({
              tx,
            }, `State transition ${stHash.toString('hex')} is passing CheckTx but removed from the block by proposal`);

            const error = new Error('State Transition processing error. Please report'
              + ' faulty state transition and try to create a new state transition with different'
              + ' hash as a workaround.');

            throw new InternalGrpcError(error);
          }
        }

        if (jsonRpcError.data.startsWith('Tx too large.')) {
          const message = jsonRpcError.data.replace('Tx too large. ', '');
          throw new InvalidArgumentGrpcError(`state transition is too large. ${message}`);
        }

        if (jsonRpcError.data.startsWith('mempool is full')) {
          throw new ResourceExhaustedGrpcError(jsonRpcError.data);
        }

        // broadcasting is timed out
        if (jsonRpcError.data.includes('context deadline exceeded')) {
          throw new ResourceExhaustedGrpcError('broadcasting state transition is timed out');
        }

        if (jsonRpcError.data.includes('too_many_resets')) {
          throw new ResourceExhaustedGrpcError('tenderdash is not responding: too many requests');
        }

        if (jsonRpcError.data.startsWith('broadcast confirmation not received:')) {
          logger.error(`Failed broadcasting state transition: ${jsonRpcError.data}`);

          throw new UnavailableGrpcError(jsonRpcError.data);
        }
      }

      const error = new Error();
      Object.assign(error, jsonRpcError);

      logger.error(error, `Unexpected JSON RPC error during broadcasting state transition: ${JSON.stringify(jsonRpcError)}`);

      throw error;
    }

    if (result.code !== 0) {
      throw await createGrpcErrorFromDriveResponse(result.code, result.info);
    }

    return new BroadcastStateTransitionResponse();
  }

  return broadcastStateTransitionHandler;
}

module.exports = broadcastStateTransitionHandlerFactory;

const {
  server: {
    error: {
      InvalidArgumentGrpcError,
      DeadlineExceededGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    WaitForStateTransitionResultResponse,
    StateTransitionBroadcastError,
  },
} = require('@dashevo/dapi-grpc');

const UnavailableGrpcError = require('@dashevo/grpc-common/lib/server/error/UnavailableGrpcError');
const TransactionWaitPeriodExceededError = require('../../../errors/TransactionWaitPeriodExceededError');
const TransactionErrorResult = require('../../../externalApis/tenderdash/waitForTransactionToBeProvable/transactionResult/TransactionErrorResult');

/**
 *
 * @param {fetchProofForStateTransition} fetchProofForStateTransition
 * @param {waitForTransactionToBeProvable} waitForTransactionToBeProvable
 * @param {BlockchainListener} blockchainListener
 * @param {createGrpcErrorFromDriveResponse} createGrpcErrorFromDriveResponse
 * @param {number} stateTransitionWaitTimeout
 * @return {waitForStateTransitionResultHandler}
 */
function waitForStateTransitionResultHandlerFactory(
  fetchProofForStateTransition,
  waitForTransactionToBeProvable,
  blockchainListener,
  createGrpcErrorFromDriveResponse,
  stateTransitionWaitTimeout,
) {
  /**
   * @param {Object} txDeliverResult
   * @return {StateTransitionBroadcastError}
   */
  async function createStateTransitionDeliverError(txDeliverResult) {
    const grpcError = await createGrpcErrorFromDriveResponse(
      txDeliverResult.code,
      txDeliverResult.info,
    );

    const error = new StateTransitionBroadcastError();

    const metadata = grpcError.getRawMetadata();
    if (metadata['dash-serialized-consensus-error-bin']) {
      error.setData(metadata['dash-serialized-consensus-error-bin']);
    }

    error.setCode(txDeliverResult.code);
    error.setMessage(grpcError.getMessage());

    return error;
  }

  /**
   * @typedef waitForStateTransitionResultHandler
   * @param {Object} call
   * @return {Promise<WaitForStateTransitionResultResponse>}
   */
  async function waitForStateTransitionResultHandler(call) {
    const { request } = call;

    if (!blockchainListener.wsClient.isConnected) {
      throw new UnavailableGrpcError('Tenderdash is not available');
    }

    const stateTransitionHash = request.getV0().getStateTransitionHash();
    const prove = request.getV0().getProve();

    if (!stateTransitionHash) {
      throw new InvalidArgumentGrpcError('state transition hash is not specified');
    }

    const hashString = Buffer.from(stateTransitionHash).toString('hex').toUpperCase();

    let result;

    try {
      result = await waitForTransactionToBeProvable(
        blockchainListener,
        hashString,
        stateTransitionWaitTimeout,
      );
    } catch (e) {
      if (e instanceof TransactionWaitPeriodExceededError) {
        throw new DeadlineExceededGrpcError(
          `Waiting period for state transition ${e.getTransactionHash()} exceeded`,
          {
            stateTransitionHash: e.getTransactionHash(),
          },
        );
      }

      throw e;
    }

    const response = new WaitForStateTransitionResultResponse();
    const v0 = new WaitForStateTransitionResultResponse
      .WaitForStateTransitionResultResponseV0();

    if (result instanceof TransactionErrorResult) {
      const error = await createStateTransitionDeliverError(
        result.getResult(),
      );

      v0.setError(error);
      response.setV0(v0);

      return response;
    }

    if (prove) {
      const stateTransitionProof = await fetchProofForStateTransition(result.getTransaction());

      v0.setMetadata(stateTransitionProof.getMetadata());
      v0.setProof(stateTransitionProof.getProof());
    }

    response.setV0(v0);

    return response;
  }

  return waitForStateTransitionResultHandler;
}

module.exports = waitForStateTransitionResultHandlerFactory;

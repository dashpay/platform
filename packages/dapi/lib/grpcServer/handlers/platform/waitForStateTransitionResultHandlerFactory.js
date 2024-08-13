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

const cbor = require('cbor');
const TransactionWaitPeriodExceededError = require('../../../errors/TransactionWaitPeriodExceededError');
const TransactionErrorResult = require('../../../externalApis/tenderdash/waitForTransactionToBeProvable/transactionResult/TransactionErrorResult');

/**
 *
 * @param {fetchProofForStateTransition} fetchProofForStateTransition
 * @param {waitForTransactionToBeProvable} waitForTransactionToBeProvable
 * @param {BlockchainListener} blockchainListener
 * @param {DashPlatformProtocol} dpp
 * @param {createGrpcErrorFromDriveResponse} createGrpcErrorFromDriveResponse
 * @param {number} stateTransitionWaitTimeout
 * @return {waitForStateTransitionResultHandler}
 */
function waitForStateTransitionResultHandlerFactory(
  fetchProofForStateTransition,
  waitForTransactionToBeProvable,
  blockchainListener,
  dpp,
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

    error.setCode(txDeliverResult.code);
    error.setMessage(grpcError.getMessage());
    error.setData(cbor.encode(grpcError.getRawMetadata()));

    return error;
  }

  /**
   * @typedef waitForStateTransitionResultHandler
   * @param {Object} call
   * @return {Promise<WaitForStateTransitionResultResponse>}
   */
  async function waitForStateTransitionResultHandler(call) {
    const { request } = call;

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
      const stateTransition = await dpp.stateTransition.createFromBuffer(
        result.getTransaction(),
        { skipValidation: true },
      );

      const stateTransitionProof = await fetchProofForStateTransition(stateTransition);
      v0.setMetadata(stateTransitionProof.getV0().getMetadata());
      v0.setProof(stateTransitionProof.getV0().getProof());
    }
    response.setV0(v0);

    return response;
  }

  return waitForStateTransitionResultHandler;
}

module.exports = waitForStateTransitionResultHandlerFactory;

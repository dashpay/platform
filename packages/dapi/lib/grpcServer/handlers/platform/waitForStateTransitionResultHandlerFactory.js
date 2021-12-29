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
    Proof,
    ResponseMetadata,
    StoreTreeProofs,
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
  stateTransitionWaitTimeout = 80000,
) {
  /**
   * @param {Object} txDeliverResult
   * @return {StateTransitionBroadcastError}
   */
  function createStateTransitionDeliverError(txDeliverResult) {
    const grpcError = createGrpcErrorFromDriveResponse(txDeliverResult.code, txDeliverResult.info);

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

    const stateTransitionHash = request.getStateTransitionHash();
    const prove = request.getProve();

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

    if (result instanceof TransactionErrorResult) {
      const error = createStateTransitionDeliverError(result.getResult());

      response.setError(error);

      return response;
    }

    if (prove) {
      const stateTransition = await dpp.stateTransition.createFromBuffer(
        result.getTransaction(),
        { skipValidation: true },
      );

      const { proof: proofObject, metadata } = await fetchProofForStateTransition(stateTransition);

      const responseMetadata = new ResponseMetadata();

      responseMetadata.setHeight(metadata.height);
      responseMetadata.setCoreChainLockedHeight(metadata.coreChainLockedHeight);

      response.setMetadata(responseMetadata);

      const proof = new Proof();
      const storeTreeProofs = new StoreTreeProofs();

      if (stateTransition.isDocumentStateTransition()) {
        storeTreeProofs.setDocumentsProof(proofObject.storeTreeProof);
      } else if (stateTransition.isIdentityStateTransition()) {
        storeTreeProofs.setIdentitiesProof(proofObject.storeTreeProof);
      } else if (stateTransition.isDataContractStateTransition()) {
        storeTreeProofs.setDataContractsProof(proofObject.storeTreeProof);
      }

      proof.setRootTreeProof(proofObject.rootTreeProof);
      proof.setStoreTreeProofs(storeTreeProofs);
      proof.setSignatureLlmqHash(proofObject.signatureLlmqHash);
      proof.setSignature(proofObject.signature);

      response.setProof(proof);
    }

    return response;
  }

  return waitForStateTransitionResultHandler;
}

module.exports = waitForStateTransitionResultHandlerFactory;

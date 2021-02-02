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
  },
} = require('@dashevo/dapi-grpc');

const cbor = require('cbor');
const TransactionWaitPeriodExceededError = require('../../../errors/TransactionWaitPeriodExceededError');

/**
 *
 * @param {DriveClient} driveClient
 * @param {BlockchainListener} blockchainListener
 * @param {DashPlatformProtocol} dpp
 * @param {number} stateTransitionWaitTimeout
 * @return {waitForStateTransitionResultHandler}
 */
function waitForStateTransitionResultHandlerFactory(
  driveClient,
  blockchainListener,
  dpp,
  stateTransitionWaitTimeout = 80000,
) {
  /**
   *
   * @param {AbstractStateTransition} stateTransition
   * @return {Promise<Object>}
   */
  async function fetchProof(stateTransition) {
    const modifiedIds = stateTransition.getModifiedDataIds();

    let proof;
    if (stateTransition.isDocumentStateTransition()) {
      ({ documentsProof: proof } = await driveClient.fetchProofs(
        { documentIds: modifiedIds },
      ));
    } else if (stateTransition.isIdentityStateTransition()) {
      ({ identitiesProof: proof } = await driveClient.fetchProofs(
        { identityIds: modifiedIds },
      ));
    } else if (stateTransition.isDataContractStateTransition()) {
      ({ dataContractsProof: proof } = await driveClient.fetchProofs(
        { dataContractIds: modifiedIds },
      ));
    }

    return proof;
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
    let data;

    try {
      data = await blockchainListener.waitForTransaction(hashString, stateTransitionWaitTimeout);
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

    const { result, tx } = data.data.value.TxResult;

    if (result && result.code !== undefined && result.code !== 0) {
      const { error: abciError } = JSON.parse(result.log);

      let errorData;
      if (abciError.data) {
        errorData = cbor.encode(abciError.data);
      }

      const error = new StateTransitionBroadcastError();

      error.setCode(result.code);
      error.setMessage(abciError.message);
      error.setData(errorData);

      response.setError(error);

      return response;
    }

    // The first block is the one with the current ST, second block is the
    // block in which st result will be included in the proof
    await blockchainListener.waitForBlocks(2);

    if (prove) {
      const stateTransition = await dpp.stateTransition.createFromBuffer(
        Buffer.from(tx, 'base64'),
        { skipValidation: true },
      );
      const proofObject = await fetchProof(stateTransition);

      const proof = new Proof();
      proof.setRootTreeProof(proofObject.rootTreeProof);
      proof.setStoreTreeProof(proofObject.storeTreeProof);

      response.setProof(proof);
    }

    return response;
  }

  return waitForStateTransitionResultHandler;
}

module.exports = waitForStateTransitionResultHandlerFactory;

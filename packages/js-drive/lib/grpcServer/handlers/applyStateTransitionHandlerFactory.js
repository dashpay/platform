const { ApplyStateTransitionResponse } = require('@dashevo/drive-grpc');
const InvalidSTPacketError = require('@dashevo/dpp/lib/stPacket/errors/InvalidSTPacketError');

const {
  server: {
    error: {
      InternalGrpcError,
      InvalidArgumentGrpcError,
      FailedPreconditionGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const StateTransition = require('../../blockchain/StateTransition');

/**
 * @param {MongoDBTransaction} stateViewTransaction
 * @param {DashPlatformProtocol} dpp
 * @param {applyStateTransition} applyStateTransition
 * @param {BlockExecutionState} blockExecutionState
 * @returns {applyStateTransitionHandler}
 */
module.exports = function applyStateTransitionHandlerFactory(
  stateViewTransaction,
  dpp,
  applyStateTransition,
  blockExecutionState,
) {
  /**
   * Apply received stPacket and stHeader to database inside transaction, opened earlier
   *
   * @typedef applyStateTransitionHandler
   * @param {Object} call
   * @returns {Promise<CommitTransactionResponse>}
   */
  async function applyStateTransitionHandler({ request }) {
    if (!stateViewTransaction.isTransactionStarted) {
      throw new FailedPreconditionGrpcError('Transaction is not started');
    }

    const blockHeight = request.getBlockHeight();
    const blockHashBinaryArray = request.getBlockHash();
    const stPacketBinaryArray = request.getStateTransitionPacket();
    const stHeaderBinaryArray = request.getStateTransitionHeader();

    if (stPacketBinaryArray === undefined) {
      throw new InvalidArgumentGrpcError('stateTransitionPacket is not specified');
    }

    if (stHeaderBinaryArray === undefined) {
      throw new InvalidArgumentGrpcError('stateTransitionHeader is not specified');
    }

    if (blockHeight === undefined) {
      throw new InvalidArgumentGrpcError('blockHeight is not specified');
    }

    if (blockHashBinaryArray === undefined) {
      throw new InvalidArgumentGrpcError('blockHash is not specified');
    }

    let stPacket;

    try {
      const stPacketHex = Buffer.from(stPacketBinaryArray).toString('hex');

      stPacket = await dpp.packet.createFromSerialized(stPacketHex);
    } catch (e) {
      if (e instanceof InvalidSTPacketError) {
        throw new InvalidArgumentGrpcError(e.message, { errors: e.getErrors() });
      }

      throw e;
    }

    let stHeader;

    try {
      stHeader = new StateTransition(Buffer.from(stHeaderBinaryArray));
    } catch (e) {
      throw new InvalidArgumentGrpcError(`Invalid "stateTransitionHeader": ${e.message}`);
    }

    const result = await dpp.packet.verify(stPacket, stHeader);
    if (!result.isValid()) {
      throw new InvalidArgumentGrpcError('Invalid "stPacket" and "stHeader"', { errors: result.getErrors() });
    }

    let svContract;

    try {
      ({ svContract } = await applyStateTransition(
        stPacket,
        stHeader,
        Buffer.from(blockHashBinaryArray).toString('hex'),
        blockHeight,
        stateViewTransaction,
      ));
    } catch (error) {
      throw new InternalGrpcError(error);
    }

    if (svContract) {
      blockExecutionState.addContract(svContract);
    }

    return new ApplyStateTransitionResponse();
  }

  return applyStateTransitionHandler;
};

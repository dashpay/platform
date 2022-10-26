/**
 * @param {ProposalBlockExecutionContextCollection} proposalBlockExecutionContextCollection
 * @param {ValidatorSet} validatorSet
 * @param {createValidatorSetUpdate} createValidatorSetUpdate
 * @param {LatestCoreChainLock} latestCoreChainLock
 * @return {rotateValidators}
 */
function rotateValidatorsFactory(
  proposalBlockExecutionContextCollection,
  validatorSet,
  createValidatorSetUpdate,
  latestCoreChainLock,
) {
  /**
   * @typedef rotateValidators
   * @param {number} height
   * @param {number} round
   * @param {BaseLogger} logger
   * @return {Promise<ValidatorSetUpdate>}
   */
  async function rotateValidators(height, round, logger) {
    const consensusLogger = logger.child({
      height: height.toString(),
      abciMethod: 'rotateValidators',
    });

    const blockExecutionContext = proposalBlockExecutionContextCollection.get(round);
    const lastCommitInfo = blockExecutionContext.getLastCommitInfo();
    const coreChainLock = latestCoreChainLock.getChainLock();

    // Rotate validators

    let validatorSetUpdate;
    const rotationEntropy = Buffer.from(lastCommitInfo.stateSignature);
    if (await validatorSet.rotate(height, coreChainLock.height, rotationEntropy)) {
      validatorSetUpdate = createValidatorSetUpdate(validatorSet);

      const { quorumHash } = validatorSet.getQuorum();

      consensusLogger.debug(
        {
          quorumHash,
        },
        `Validator set switched to ${quorumHash} quorum`,
      );
    }

    return validatorSetUpdate;
  }

  return rotateValidators;
}

module.exports = rotateValidatorsFactory;

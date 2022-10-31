/**
 * @param {ProposalBlockExecutionContextCollection} proposalBlockExecutionContextCollection
 * @param {ValidatorSet} validatorSet
 * @param {createValidatorSetUpdate} createValidatorSetUpdate
 * @param {LatestCoreChainLock} latestCoreChainLock
 * @return {rotateValidatorSetUpdate}
 */
function rotateValidatorSetUpdateFactory(
  proposalBlockExecutionContextCollection,
  validatorSet,
  createValidatorSetUpdate,
  latestCoreChainLock,
) {
  /**
   * @typedef rotateValidatorSetUpdate
   * @param {number} height
   * @param {number} round
   * @param {BaseLogger} consensusLogger
   * @return {Promise<ValidatorSetUpdate>}
   */
  async function rotateValidatorSetUpdate(height, round, consensusLogger) {
    const proposalBlockExecutionContext = proposalBlockExecutionContextCollection.get(round);
    const lastCommitInfo = proposalBlockExecutionContext.getLastCommitInfo();
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

  return rotateValidatorSetUpdate;
}

module.exports = rotateValidatorSetUpdateFactory;

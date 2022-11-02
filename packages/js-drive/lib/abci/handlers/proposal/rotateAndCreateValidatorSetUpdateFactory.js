/**
 * @param {ProposalBlockExecutionContextCollection} proposalBlockExecutionContextCollection
 * @param {ValidatorSet} validatorSet
 * @param {createValidatorSetUpdate} createValidatorSetUpdate
 * @return {rotateAndCreateValidatorSetUpdate}
 */
function rotateAndCreateValidatorSetUpdateFactory(
  proposalBlockExecutionContextCollection,
  validatorSet,
  createValidatorSetUpdate,
) {
  /**
   * @typedef rotateAndCreateValidatorSetUpdate
   * @param {number} height
   * @param {number} coreChainLockedHeight
   * @param {number} round
   * @param {BaseLogger} consensusLogger
   * @return {Promise<ValidatorSetUpdate>}
   */
  async function rotateAndCreateValidatorSetUpdate(
    height,
    coreChainLockedHeight,
    round,
    consensusLogger,
  ) {
    const proposalBlockExecutionContext = proposalBlockExecutionContextCollection.get(round);
    const lastCommitInfo = proposalBlockExecutionContext.getLastCommitInfo();

    // Rotate validators

    let validatorSetUpdate;
    const rotationEntropy = Buffer.from(lastCommitInfo.stateSignature);
    if (await validatorSet.rotate(height, coreChainLockedHeight, rotationEntropy)) {
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

  return rotateAndCreateValidatorSetUpdate;
}

module.exports = rotateAndCreateValidatorSetUpdateFactory;

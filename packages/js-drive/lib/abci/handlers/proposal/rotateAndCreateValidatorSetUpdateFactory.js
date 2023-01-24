/**
 * @param {BlockExecutionContext} proposalBlockExecutionContext
 * @param {ValidatorSet} validatorSet
 * @param {createValidatorSetUpdate} createValidatorSetUpdate
 * @return {rotateAndCreateValidatorSetUpdate}
 */
function rotateAndCreateValidatorSetUpdateFactory(
  proposalBlockExecutionContext,
  validatorSet,
  createValidatorSetUpdate,
) {
  /**
   * @typedef rotateAndCreateValidatorSetUpdate
   * @param {number} height
   * @param {number} coreChainLockedHeight
   * @param {number} round
   * @param {BaseLogger} contextLogger
   * @return {Promise<ValidatorSetUpdate>}
   */
  async function rotateAndCreateValidatorSetUpdate(
    height,
    coreChainLockedHeight,
    round,
    contextLogger,
  ) {
    const lastCommitInfo = proposalBlockExecutionContext.getLastCommitInfo();

    // Rotate validators

    let validatorSetUpdate;
    const rotationEntropy = Buffer.from(lastCommitInfo.blockSignature);
    if (await validatorSet.rotate(height, coreChainLockedHeight, rotationEntropy)) {
      validatorSetUpdate = createValidatorSetUpdate(validatorSet);

      const { quorumHash } = validatorSet.getQuorum();

      contextLogger.debug(
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

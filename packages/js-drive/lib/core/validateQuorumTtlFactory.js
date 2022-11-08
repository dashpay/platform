/**
 *
 * @param {RpcClient} coreRpcClient
 * @return {validateQuorumTtl}
 */
function validateQuorumTtlFactory(coreRpcClient) {
  /**
   * @typedef validateQuorumTtl
   * @param {SimplifiedMNList} sml
   * @param {number} quorumType
   * @param {QuorumEntry} quorum
   * @param {number} coreHeight
   * @param {number} blockRotationInterval
   * @return {Promise<boolean>}
   */
  async function validateQuorumTtl(sml, quorumType, quorum, coreHeight, blockRotationInterval) {
    const block = await coreRpcClient.getBlock(quorum.quorumHash);

    const minTtl = blockRotationInterval * 3;
    const dkgInterval = 24;
    const numberOfQuorums = sml.getQuorumsOfType(quorumType).length;
    const quorumRemoveHeight = block.height + (dkgInterval * numberOfQuorums);
    const howMuchInRest = quorumRemoveHeight - coreHeight;
    const quorumTtl = howMuchInRest * 2.5;

    return quorumTtl > minTtl;
  }

  return validateQuorumTtl;
}

module.exports = validateQuorumTtlFactory;

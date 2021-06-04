/**
 * @param {RpcClient} coreRpcClient
 * @return {fetchQuorumMembers}
 */
function fetchQuorumMembersFactory(coreRpcClient) {
  /**
   * @typedef {fetchQuorumMembers}
   * @param {number} quorumType
   * @param {string} quorumHash
   * @return {Promise<Object[]>}
   */
  async function fetchQuorumMembers(quorumType, quorumHash) {
    try {
      const {
        result: {
          members: validators,
        },
      } = await coreRpcClient.quorum(
        'info',
        quorumType,
        quorumHash,
      );

      return validators;
    } catch (e) {
      // RPC_INVALID_PARAMETER: quorum not found
      if (e.code === -8) {
        throw new Error(`The quorum of type ${quorumType} and quorumHash ${quorumHash} doesn't exist`);
      }

      throw e;
    }
  }

  return fetchQuorumMembers;
}

module.exports = fetchQuorumMembersFactory;

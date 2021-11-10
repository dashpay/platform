/**
 * @param {DriveClient} driveClient
 * @return {fetchProofForStateTransition}
 */
function fetchProofForStateTransitionFactory(driveClient) {
  /**
   * @typedef {fetchProofForStateTransition}
   * @param {AbstractStateTransition} stateTransition
   * @return {Promise<Object>}
   */
  async function fetchProofForStateTransition(stateTransition) {
    const modifiedIds = stateTransition.getModifiedDataIds();

    let proof;
    let metadata;
    if (stateTransition.isDocumentStateTransition()) {
      ({ documentsProof: proof, metadata } = await driveClient.fetchProofs(
        { documentIds: modifiedIds.map(Buffer.from) },
      ));
    } else if (stateTransition.isIdentityStateTransition()) {
      ({ identitiesProof: proof, metadata } = await driveClient.fetchProofs(
        { identityIds: modifiedIds.map(Buffer.from) },
      ));
    } else if (stateTransition.isDataContractStateTransition()) {
      ({ dataContractsProof: proof, metadata } = await driveClient.fetchProofs(
        { dataContractIds: modifiedIds.map(Buffer.from) },
      ));
    }

    return { proof, metadata };
  }

  return fetchProofForStateTransition;
}

module.exports = fetchProofForStateTransitionFactory;

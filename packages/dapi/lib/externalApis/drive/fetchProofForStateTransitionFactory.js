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

  return fetchProofForStateTransition;
}

module.exports = fetchProofForStateTransitionFactory;

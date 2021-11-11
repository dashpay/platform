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
        {
          documentIds: modifiedIds.map((identifier) => identifier.toBuffer()),
        },
      ));
    } else if (stateTransition.isIdentityStateTransition()) {
      ({ identitiesProof: proof, metadata } = await driveClient.fetchProofs(
        {
          identityIds: modifiedIds((identifier) => identifier.toBuffer()),
        },
      ));
    } else if (stateTransition.isDataContractStateTransition()) {
      ({ dataContractsProof: proof, metadata } = await driveClient.fetchProofs(
        {
          dataContractIds: modifiedIds.map((identifier) => identifier.toBuffer()),
        },
      ));
    }

    return { proof, metadata };
  }

  return fetchProofForStateTransition;
}

module.exports = fetchProofForStateTransitionFactory;

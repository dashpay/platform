const {
  v0: {
    GetProofsRequest,
    GetProofsResponse,
  },
} = require('@dashevo/dapi-grpc');

/**
 * @param {DriveClient} driveClient
 * @return {fetchProofForStateTransition}
 */
function fetchProofForStateTransitionFactory(driveClient) {
  /**
   * @typedef {fetchProofForStateTransition}
   * @param {AbstractStateTransition} stateTransition
   * @return {Promise<GetProofsResponse>}
   */
  async function fetchProofForStateTransition(stateTransition) {
    const getProofsRequest = new GetProofsRequest();

    const modifiedIds = stateTransition.getModifiedDataIds();

    if (stateTransition.isDocumentStateTransition()) {
      const { DocumentProofRequest } = GetProofsRequest;

      const documentsList = stateTransition.getTransitions().map((documentTransition) => {
        const documentRequest = new DocumentProofRequest();
        documentRequest.setContractId(documentTransition.getDataContractId().toBuffer());
        documentRequest.setDocumentType(documentTransition.getType());
        documentRequest.setDocumentId(documentTransition.getId().toBuffer());
        return documentRequest;
      });

      getProofsRequest.setDocumentsList(documentsList);
    } if (stateTransition.isIdentityStateTransition()) {
      const { IdentityRequest } = GetProofsRequest;

      getProofsRequest.setIdentitiesList(modifiedIds.map((id) => {
        const identityRequest = new IdentityRequest();
        identityRequest.setIdentityId(id.toBuffer());
        identityRequest.setRequestType(IdentityRequest.Type.FULL_IDENTITY);
        return identityRequest;
      }));
    } if (stateTransition.isDataContractStateTransition()) {
      const { ContractRequest } = GetProofsRequest;

      getProofsRequest.setContractsList(modifiedIds.map((id) => {
        const identityRequest = new ContractRequest();
        identityRequest.setContractId(id.toBuffer());
        return identityRequest;
      }));
    }

    const responseBytes = await driveClient.fetchProofs(getProofsRequest);
    return GetProofsResponse.deserializeBinary(responseBytes);
  }

  return fetchProofForStateTransition;
}

module.exports = fetchProofForStateTransitionFactory;

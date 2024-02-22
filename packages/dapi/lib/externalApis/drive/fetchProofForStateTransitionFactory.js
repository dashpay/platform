const {
  v0: {
    GetProofsRequest,
    GetProofsResponse,
  },
} = require('@dashevo/dapi-grpc');
const { StateTransitionTypes } = require('@dashevo/wasm-dpp');

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

    const { GetProofsRequestV0 } = GetProofsRequest;

    if (stateTransition.isDocumentStateTransition()) {
      const { DocumentRequest } = GetProofsRequestV0;

      const documentsList = stateTransition.getTransitions().map((documentTransition) => {
        const documentRequest = new DocumentRequest();
        documentRequest.setContractId(documentTransition.getDataContractId().toBuffer());
        documentRequest.setDocumentType(documentTransition.getType());
        documentRequest.setDocumentId(documentTransition.getId().toBuffer());
        return documentRequest;
      });

      getProofsRequest.setV0(new GetProofsRequestV0().setDocumentsList(documentsList));
    } if (stateTransition.isIdentityStateTransition()) {
      const { IdentityRequest } = GetProofsRequestV0;

      const identitiesList = modifiedIds.flatMap((id) => {
        const stType = stateTransition.getType();
        let proofRequests;

        if (stType === StateTransitionTypes.IdentityCreditTransfer) {
          proofRequests = new IdentityRequest();
          proofRequests.setIdentityId(id.toBuffer());
          proofRequests.setRequestType(IdentityRequest.Type.BALANCE);
        } else if (stType === StateTransitionTypes.IdentityTopUp) {
          const proofRequestsBalance = new IdentityRequest();
          proofRequestsBalance.setIdentityId(id.toBuffer());
          proofRequestsBalance.setRequestType(IdentityRequest.Type.BALANCE);

          const proofRequestsRevision = new IdentityRequest();
          proofRequestsRevision.setIdentityId(id.toBuffer());
          proofRequestsRevision.setRequestType(IdentityRequest.Type.REVISION);

          proofRequests = [proofRequestsBalance, proofRequestsRevision];
        } else {
          proofRequests = new IdentityRequest();
          proofRequests.setIdentityId(id.toBuffer());
          proofRequests.setRequestType(IdentityRequest.Type.FULL_IDENTITY);
        }

        return proofRequests;
      });

      getProofsRequest.setV0(new GetProofsRequestV0().setIdentitiesList(identitiesList));
    } if (stateTransition.isDataContractStateTransition()) {
      const { ContractRequest } = GetProofsRequestV0;

      const contractsList = modifiedIds.map((id) => {
        const identityRequest = new ContractRequest();
        identityRequest.setContractId(id.toBuffer());
        return identityRequest;
      });

      getProofsRequest.setV0(new GetProofsRequestV0()
        .setContractsList(contractsList));
    }

    const responseBytes = await driveClient.fetchProofs(getProofsRequest);
    return GetProofsResponse.deserializeBinary(responseBytes);
  }

  return fetchProofForStateTransition;
}

module.exports = fetchProofForStateTransitionFactory;

const {
  v0: {
    GetProofsRequest,
  },
} = require('@dashevo/dapi-grpc');
const { StateTransitionTypes } = require('@dashevo/wasm-dpp');

/**
 * @param {PlatformPromiseClient} driveClient
 * @return {fetchProofForStateTransition}
 */
function fetchProofForStateTransitionFactory(driveClient) {
  /**
   * @typedef {fetchProofForStateTransition}
   * @param {AbstractStateTransition} stateTransition
   * @return {Promise<GetProofsResponse>}
   */
  async function fetchProofForStateTransition(stateTransition) {
    const modifiedIds = stateTransition.getModifiedDataIds();

    const { GetProofsRequestV0 } = GetProofsRequest;

    const requestV0 = new GetProofsRequestV0();

    if (stateTransition.isDocumentStateTransition()) {
      const { DocumentRequest } = GetProofsRequestV0;

      const documentsList = stateTransition.getTransitions().map((documentTransition) => {
        const documentRequest = new DocumentRequest();
        documentRequest.setContractId(documentTransition.getDataContractId().toBuffer());
        documentRequest.setDocumentType(documentTransition.getType());
        documentRequest.setDocumentId(documentTransition.getId().toBuffer());
        return documentRequest;
      });

      requestV0.setDocumentsList(documentsList);
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

      requestV0.setIdentitiesList(identitiesList);
    } if (stateTransition.isDataContractStateTransition()) {
      const { ContractRequest } = GetProofsRequestV0;

      const contractsList = modifiedIds.map((id) => {
        const identityRequest = new ContractRequest();
        identityRequest.setContractId(id.toBuffer());
        return identityRequest;
      });

      requestV0.setContractsList(contractsList);
    }

    const request = new GetProofsRequest();
    request.setV0(requestV0);

    return driveClient.getProofs(request);
  }

  return fetchProofForStateTransition;
}

module.exports = fetchProofForStateTransitionFactory;

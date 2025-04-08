const {
  v0: {
    GetProofsRequest,
  },
} = require('@dashevo/dapi-grpc');

const {
  StateTransitionTypes,
  TokenTransition,
  DocumentTransition,
  TokenTransitionType,
} = require('@dashevo/wasm-dpp');
const { GetDataContractRequest } = require('@dashevo/dapi-grpc/clients/platform/v0/web/platform_pb');
const { contractId: tokensHistoryContractIdString } = require('@dashevo/token-history-contract/lib/systemIds');
const bs58 = require('bs58');

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
} if (stateTransition.isVotingStateTransition()) {
  const { VoteStatusRequest } = GetProofsRequestV0;
  const { ContestedResourceVoteStatusRequest } = VoteStatusRequest;

  const contestedResourceVoteStatusRequest = new ContestedResourceVoteStatusRequest();

  const contestedVotePoll = stateTransition.getContestedDocumentResourceVotePoll();

  if (!contestedVotePoll) {
    throw new Error('Masternode vote state transition should have a contested vote poll');
  }

  contestedResourceVoteStatusRequest.setContractId(contestedVotePoll.contractId.toBuffer());
  contestedResourceVoteStatusRequest.setDocumentTypeName(contestedVotePoll.documentTypeName);
  contestedResourceVoteStatusRequest.setIndexName(contestedVotePoll.indexName);
  contestedResourceVoteStatusRequest.setIndexValuesList(contestedVotePoll.indexValues);
  contestedResourceVoteStatusRequest.setVoterIdentifier(
    stateTransition.getProTxHash().toBuffer(),
  );

  const voteStatus = new VoteStatusRequest();
  voteStatus.setContestedResourceVoteStatusRequest(contestedResourceVoteStatusRequest);

  requestV0.setVotesList([voteStatus]);
}

/**
 * @param {PlatformPromiseClient} driveClient
 * @param {DashPlatformProtocol} dpp
 * @return {fetchProofForStateTransition}
 */
function fetchProofForStateTransitionFactory(driveClient, dpp) {
  const tokensHistoryContractIdBuffer = bs58.decode(tokensHistoryContractIdString);


  /**
   * @typedef {fetchProofForStateTransition}
   * @param {Uint8Array} stateTransition
   * @return {Promise<GetProofsResponse>}
   */
  async function fetchProofForStateTransition(stateTransition) {
    const request = new GetProofsRequest();

    request.setStateTransition(stateTransition);

    return driveClient.getProofs(request);
  }

  return fetchProofForStateTransition;
}

module.exports = fetchProofForStateTransitionFactory;

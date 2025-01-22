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

/**
 * @param {PlatformPromiseClient} driveClient
 * @param {DashPlatformProtocol} dpp
 * @return {fetchProofForStateTransition}
 */
function fetchProofForStateTransitionFactory(driveClient, dpp) {
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
      const {
        DocumentRequest,
        IdentityTokenBalanceRequest,
        IdentityTokenInfoRequest,
        TokenStatusRequest,
      } = GetProofsRequestV0;

      const documentsList = [];
      const identityTokenBalancesList = [];
      const identityTokenInfosList = [];
      const tokenStatusesList = [];

      for (const batchedTransition of stateTransition.getTransitions()) {
        if (batchedTransition instanceof TokenTransition) {
          switch (batchedTransition.getTransitionType()) {
            case TokenTransitionType.Burn: {
              const request = new IdentityTokenBalanceRequest({
                tokenId: batchedTransition.getTokenId()
                  .toBuffer(),
                identityId: stateTransition.getOwnerId()
                  .toBuffer(),
              });

              identityTokenBalancesList.push(request);
              break;
            }
            case TokenTransitionType.Mint: {
              // Fetch data contract to determine correct recipient identity
              const dataContractId = batchedTransition.getDataContractId();

              const dataContractRequestV0 = new GetDataContractRequest.GetDataContractRequestV0({
                id: dataContractId.toBuffer(),
              });

              const dataContractRequest = new GetDataContractRequest();
              dataContractRequest.setV0(dataContractRequestV0);

              const dataContractResponse = await driveClient.getDataContract(dataContractRequest);

              const dataContractBuffer = Buffer.from(
                dataContractResponse.getV0().getDataContract_asU8(),
              );

              const dataContract = await dpp.dataContract.createFromBuffer(dataContractBuffer);

              const tokenConfiguration = dataContract.getTokenConfiguration(
                batchedTransition.getTokenContractPosition(),
              );

              const request = new IdentityTokenBalanceRequest({
                tokenId: batchedTransition.getTokenId()
                  .toBuffer(),
                identityId: batchedTransition.toTransition().getRecipientId(tokenConfiguration)
                  .toBuffer(),
              });

              identityTokenBalancesList.push(request);
              break;
            }
            case TokenTransitionType.Transfer: {
              const requestSender = new IdentityTokenBalanceRequest({
                tokenId: batchedTransition.getTokenId()
                  .toBuffer(),
                identityId: stateTransition.getOwnerId().toBuffer(),
              });

              const requestRecipient = new IdentityTokenBalanceRequest({
                tokenId: batchedTransition.getTokenId()
                  .toBuffer(),
                identityId: batchedTransition.toTransition().getRecipientId()
                  .toBuffer(),
              });

              identityTokenBalancesList.push(requestSender, requestRecipient);
              break;
            }
            case TokenTransitionType.DestroyFrozenFunds: {
              const request = new IdentityTokenBalanceRequest({
                tokenId: batchedTransition.getTokenId()
                  .toBuffer(),
                identityId: batchedTransition.toTransition().getFrozenIdentityId()
                  .toBuffer(),
              });

              identityTokenBalancesList.push(request);
              break;
            }
            case TokenTransitionType.EmergencyAction:
            {
              const request = new TokenStatusRequest({
                tokenId: batchedTransition.getTokenId()
                  .toBuffer(),
              });

              tokenStatusesList.push(request);
              break;
            }
            case TokenTransitionType.Freeze:
            case TokenTransitionType.Unfreeze: {
              const request = new IdentityTokenInfoRequest({
                tokenId: batchedTransition.getTokenId()
                  .toBuffer(),
                identityId: batchedTransition.toTransition().getFrozenIdentityId()
                  .toBuffer(),
              });

              identityTokenInfosList.push(request);
              break;
            }
            default:
              throw new Error(`Unsupported token transition type ${batchedTransition.getTransitionType()}`);
          }
        } else if (batchedTransition instanceof DocumentTransition) {
          const documentRequest = new DocumentRequest();
          documentRequest.setContractId(batchedTransition.getDataContractId().toBuffer());
          documentRequest.setDocumentType(batchedTransition.getType());
          documentRequest.setDocumentId(batchedTransition.getId().toBuffer());

          const status = batchedTransition.hasPrefundedBalance()
            ? DocumentRequest.DocumentContestedStatus.CONTESTED
            : DocumentRequest.DocumentContestedStatus.NOT_CONTESTED;

          documentRequest.setDocumentContestedStatus(status);

          documentsList.push(documentRequest);
        } else {
          throw new Error(`Unsupported batched transition type ${batchedTransition.constructor.name}`);
        }
      }

      if (documentsList.length > 0) {
        requestV0.setDocumentsList(documentsList);
      }

      if (identityTokenBalancesList.length > 0) {
        requestV0.setIdentityTokenBalancesList(identityTokenBalancesList);
      }

      if (identityTokenInfosList.length > 0) {
        requestV0.setIdentityTokenInfosList(identityTokenInfosList);
      }

      if (tokenStatusesList.length > 0) {
        requestV0.setTokenStatusesList(tokenStatusesList);
      }
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

    const request = new GetProofsRequest();
    request.setV0(requestV0);

    return driveClient.getProofs(request);
  }

  return fetchProofForStateTransition;
}

module.exports = fetchProofForStateTransitionFactory;

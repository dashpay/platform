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

function keepsHistory(batchedTransition, tokenConfiguration) {
  switch (batchedTransition.getTransitionType()) {
    case TokenTransitionType.Burn: {
      return tokenConfiguration.keepsHistory().keepsBurningHistory();
    }
    case TokenTransitionType.Mint: {
      return tokenConfiguration.keepsHistory().keepsMintingHistory();
    }
    case TokenTransitionType.Transfer: {
      return tokenConfiguration.keepsHistory().keepsTransferHistory();
    }
    case TokenTransitionType.Freeze:
    case TokenTransitionType.Unfreeze: {
      return tokenConfiguration.keepsHistory().keepsFreezingHistory();
    }
    case TokenTransitionType.Claim:
    case TokenTransitionType.ConfigUpdate:
    case TokenTransitionType.EmergencyAction:
    case TokenTransitionType.DestroyFrozenFunds: {
      return true;
    }
    default:
      return false;
  }
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
   * @param {AbstractStateTransition} stateTransition
   * @return {Promise<GetProofsResponse>}
   */
  async function fetchProofForStateTransition(stateTransition) {
    const modifiedIds = stateTransition.getModifiedDataIds();

    const { GetProofsRequestV0 } = GetProofsRequest;

    const requestV0 = new GetProofsRequestV0();

    const dataContractsCache = {};

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
        // Fetch data contract to determine correct recipient identity
        const dataContractId = batchedTransition.getDataContractId();
        const dataContractIdString = dataContractId.toString();

        if (batchedTransition instanceof TokenTransition) {
          if (!dataContractsCache[dataContractIdString]) {
            const dataContractRequestV0 = new GetDataContractRequest.GetDataContractRequestV0();
            dataContractRequestV0.setId(dataContractId.toBuffer());

            const dataContractRequest = new GetDataContractRequest();
            dataContractRequest.setV0(dataContractRequestV0);

            const dataContractResponse = await driveClient.getDataContract(dataContractRequest);

            const dataContractBuffer = Buffer.from(
              dataContractResponse.getV0().getDataContract_asU8(),
            );

            dataContractsCache[dataContractIdString] = await dpp.dataContract
              .createFromBuffer(dataContractBuffer, { skipValidation: true });
          }

          const dataContract = dataContractsCache[dataContractIdString];

          const tokenConfiguration = dataContract.getTokenConfiguration(
            batchedTransition.getTokenContractPosition(),
          );

          // In case if we keep history for token events we can provide better proof
          // for clients
          if (keepsHistory(batchedTransition, tokenConfiguration)) {
            const documentRequest = new DocumentRequest();
            documentRequest.setContractId(tokensHistoryContractIdBuffer);
            documentRequest.setDocumentType(batchedTransition.getHistoricalDocumentTypeName());

            const documentId = batchedTransition.getHistoricalDocumentId(
              stateTransition.getOwnerId(),
              batchedTransition.getIdentityContractNonce(),
            );

            documentRequest.setDocumentId(documentId.toBuffer());

            documentsList.push(documentRequest);
          } else {
            // If not we can provide only balance / supply proofs
            switch (batchedTransition.getTransitionType()) {
              case TokenTransitionType.Burn: {
                const request = new IdentityTokenBalanceRequest();
                request.setTokenId(batchedTransition.getTokenId().toBuffer());
                request.setIdentityId(stateTransition.getOwnerId().toBuffer());

                identityTokenBalancesList.push(request);
                break;
              }
              case TokenTransitionType.Mint: {
                const request = new IdentityTokenBalanceRequest();
                request.setTokenId(batchedTransition.getTokenId().toBuffer());
                request.setIdentityId(
                  batchedTransition.toTransition().getRecipientId(tokenConfiguration).toBuffer(),
                );

                identityTokenBalancesList.push(request);
                break;
              }
              case TokenTransitionType.Transfer: {
                const requestSender = new IdentityTokenBalanceRequest();
                requestSender.setTokenId(batchedTransition.getTokenId().toBuffer());
                requestSender.setIdentityId(stateTransition.getOwnerId().toBuffer());

                const requestRecipient = new IdentityTokenBalanceRequest();
                requestRecipient.setTokenId(batchedTransition.getTokenId().toBuffer());
                requestRecipient.setIdentityId(
                  batchedTransition.toTransition().getRecipientId().toBuffer(),
                );

                identityTokenBalancesList.push(requestSender, requestRecipient);
                break;
              }
              case TokenTransitionType.EmergencyAction:
              {
                const request = new TokenStatusRequest();

                request.setTokenId(batchedTransition.getTokenId().toBuffer());

                tokenStatusesList.push(request);
                break;
              }
              case TokenTransitionType.Freeze:
              case TokenTransitionType.Unfreeze: {
                const request = new IdentityTokenInfoRequest();
                request.setTokenId(batchedTransition.getTokenId().toBuffer());
                request.setIdentityId(
                  batchedTransition.toTransition().getFrozenIdentityId().toBuffer(),
                );

                identityTokenInfosList.push(request);
                break;
              }
              default:
                throw new Error(`Unsupported token transition type ${batchedTransition.getTransitionType()}`);
            }
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

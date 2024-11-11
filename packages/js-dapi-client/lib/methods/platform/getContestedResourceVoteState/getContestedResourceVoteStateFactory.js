const {
  v0: {
    PlatformPromiseClient,
    GetContestedResourceVoteStateRequest,
  },
} = require('@dashevo/dapi-grpc');

const GetContestedResourceVoteStateResponse = require('./GetContestedResourceVoteStateResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getContestedResourceVoteStateRequest}
 */
function getContestedResourceVoteStateFactory(grpcTransport) {
  /**
   * Fetch the version upgrade votes status
   * @typedef {getContestedResourceVoteState}
   * @param contractId
   * @param documentTypeName
   * @param indexName
   * @param resultType
   * @param indexValuesList
   * @param startAtIdentifierInfo
   * @param allowIncludeLockedAndAbstainingVoteTally
   * @param count
   * @param {DAPIClientOptions & {prove: boolean}} [options]
   * @returns {Promise<gрetContestedResourceVoteStateResponse>}
   */
  async function getContestedResourceVoteState(
    contractId,
    documentTypeName,
    indexName,
    resultType,
    indexValuesList,
    startAtIdentifierInfo,
    allowIncludeLockedAndAbstainingVoteTally,
    count,
    options = {},
  ) {
    const { GetContestedResourceVoteStateRequestV0 } = GetContestedResourceVoteStateRequest;

    // eslint-disable-next-line max-len
    const getContestedResourceVoteStateRequest = new GetContestedResourceVoteStateRequest();

    if (Buffer.isBuffer(contractId)) {
      // eslint-disable-next-line no-param-reassign
      contractId = Buffer.from(contractId);
    }

    getContestedResourceVoteStateRequest.setV0(
      new GetContestedResourceVoteStateRequestV0()
        .setContractId(contractId)
        .setDocumentTypeName(documentTypeName)
        .setIndexName(indexName)
        .setResultType(resultType)
        .setIndexValuesList(indexValuesList)
        .setStartAtIdentifierInfo(startAtIdentifierInfo)
        .setAllowIncludeLockedAndAbstainingVoteTally(allowIncludeLockedAndAbstainingVoteTally)
        .setCount(count)
        .setProve(!!options.prove),
    );

    let lastError;

    // TODO: simple retry before the dapi versioning is properly implemented
    for (let i = 0; i < 3; i += 1) {
      try {
        // eslint-disable-next-line no-await-in-loop
        const getContestedResourceVoteStateResponse = await grpcTransport.request(
          PlatformPromiseClient,
          'getContestedResourceVoteState',
          getContestedResourceVoteStateRequest,
          options,
        );

        return GetContestedResourceVoteStateResponse
          .createFromProto(getContestedResourceVoteStateResponse);
      } catch (e) {
        if (e instanceof InvalidResponseError) {
          lastError = e;
        } else {
          throw e;
        }
      }
    }

    // If we made it past the cycle it means that the retry didn't work,
    // and we're throwing the last error encountered
    throw lastError;
  }

  return getContestedResourceVoteState;
}

module.exports = getContestedResourceVoteStateFactory;

const {
  v0: {
    PlatformPromiseClient,
    GetContestedResourcesRequest,
  },
} = require('@dashevo/dapi-grpc');

const GetContestedResourcesResponse = require('./GetContestedResourceResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getContestedResourcesRequest}
 */
function getContestedResourcesFactory(grpcTransport) {
  /**
   * Fetch the contested resources for specific contract
   * @typedef {getContestedResources}
   * @param contractId
   * @param documentTypeName
   * @param indexName
   * @param startIndexValues
   * @param endIndexValues
   * @param startAtValueInfo
   * @param count
   * @param orderAscending
   * @param {DAPIClientOptions & {prove: boolean}} [options]
   * @returns {Promise<getContestedResourcesResponse>}
   */
  async function getContestedResources(
    contractId,
    documentTypeName,
    indexName,
    startIndexValues,
    endIndexValues,
    startAtValueInfo,
    count,
    orderAscending,
    options = {},
  ) {
    const { GetContestedResourcesRequestV0 } = GetContestedResourcesRequest;

    // eslint-disable-next-line max-len
    const getContestedResourcesRequest = new GetContestedResourcesRequest();

    if (Buffer.isBuffer(contractId)) {
      // eslint-disable-next-line no-param-reassign
      contractId = Buffer.from(contractId);
    }

    getContestedResourcesRequest.setV0(
      new GetContestedResourcesRequestV0()
        .setContractId(contractId)
        .setDocumentTypeName(documentTypeName)
        .setIndexName(indexName)
        .setStartIndexValuesList(startIndexValues)
        .setEndIndexValuesList(endIndexValues)
        .setStartAtValueInfo(startAtValueInfo)
        .setCount(count)
        .setOrderAscending(orderAscending)
        .setProve(!!options.prove),
    );

    let lastError;

    // TODO: simple retry before the dapi versioning is properly implemented
    for (let i = 0; i < 3; i += 1) {
      try {
        // eslint-disable-next-line no-await-in-loop
        const getContestedResourcesResponse = await grpcTransport.request(
          PlatformPromiseClient,
          'getContestedResources',
          getContestedResourcesRequest,
          options,
        );

        return GetContestedResourcesResponse
          .createFromProto(getContestedResourcesResponse);
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

  return getContestedResources;
}

module.exports = getContestedResourcesFactory;

const cbor = require('cbor');

const {
  v0: {
    PlatformPromiseClient,
    GetDocumentsRequest,
  },
} = require('@dashevo/dapi-grpc');

const GetDocumentsResponse = require('./GetDocumentsResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getDocuments}
 */
function getDocumentsFactory(grpcTransport) {
  /**
   * Fetch Documents from Drive
   *
   * @typedef {getDocuments}
   * @param {Buffer} contractId - Data Contract ID
   * @param {string} type - Document type
   * @param {DAPIClientOptions & getDocumentsOptions & {prove: boolean}} [options]
   * @returns {Promise<GetDocumentsResponse>}
   */
  async function getDocuments(contractId, type, options = {}) {
    const {
      where,
      orderBy,
      limit,
      startAt,
      startAfter,
    } = options;

    let whereSerialized;
    if (where) {
      whereSerialized = cbor.encode(where);
    }

    let orderBySerialized;
    if (orderBy) {
      orderBySerialized = cbor.encode(orderBy);
    }

    const getDocumentsRequest = new GetDocumentsRequest();
    // need to convert Identifier to pure buffer as google protobuf doesn't support extended buffers
    // https://github.com/protocolbuffers/protobuf/blob/master/js/binary/utils.js#L1049

    // need to convert objects inherited from Buffer to pure buffer as google protobuf
    // doesn't support extended buffers
    // https://github.com/protocolbuffers/protobuf/blob/master/js/binary/utils.js#L1049
    if (Buffer.isBuffer(contractId)) {
      // eslint-disable-next-line no-param-reassign
      contractId = Buffer.from(contractId);
    }

    getDocumentsRequest.setDataContractId(contractId);
    getDocumentsRequest.setDocumentType(type);
    getDocumentsRequest.setWhere(whereSerialized);
    getDocumentsRequest.setOrderBy(orderBySerialized);
    getDocumentsRequest.setLimit(limit);
    getDocumentsRequest.setStartAfter(startAfter);
    getDocumentsRequest.setStartAt(startAt);
    getDocumentsRequest.setProve(!!options.prove);

    let lastError;

    // TODO: simple retry before the dapi versioning is properly implemented
    for (let i = 0; i < 3; i += 1) {
      try {
        // eslint-disable-next-line no-await-in-loop
        const getDocumentsResponse = await grpcTransport.request(
          PlatformPromiseClient,
          'getDocuments',
          getDocumentsRequest,
          options,
        );

        return GetDocumentsResponse.createFromProto(getDocumentsResponse);
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

  return getDocuments;
}

/**
 * @typedef {object} getDocumentsOptions
 * @property {object} [where]
 * @property {object} [orderBy]
 * @property {object} [limit]
 * @property {object} [startAt]
 * @property {object} [startAfter]
 */

module.exports = getDocumentsFactory;

const cbor = require('cbor');

const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    GetDocumentsResponse,
  },
} = require('@dashevo/dapi-grpc');

/**
 *
 * @param {DriveClient} driveClient
 *
 * @returns {getDocumentsHandler}
 */
function getDocumentsHandlerFactory(driveClient) {
  /**
   * @typedef getDocumentsHandler
   *
   * @param {Object} call
   *
   * @returns {Promise<GetDocumentsResponse>}
   */
  async function getDocumentsHandler(call) {
    const { request } = call;

    // Data Contract ID
    const dataContractId = request.getDataContractId();

    if (!dataContractId) {
      throw new InvalidArgumentGrpcError('dataContractId is not specified');
    }

    // Documents type

    const documentType = request.getDocumentType();

    if (!documentType) {
      throw new InvalidArgumentGrpcError('documentType is not specified');
    }

    // Where

    const whereBinary = request.getWhere_asU8();

    let where;
    if (whereBinary.length > 0) {
      where = cbor.decode(
        Buffer.from(whereBinary),
      );
    }

    // Order by

    const orderByBinary = request.getOrderBy_asU8();

    let orderBy;
    if (orderByBinary.length > 0) {
      orderBy = cbor.decode(
        Buffer.from(orderByBinary),
      );
    }

    // Limit

    const limitOrDefault = request.getLimit();

    let limit;
    if (limitOrDefault !== 0) {
      limit = limitOrDefault;
    }

    // Start after

    const startAfterBinary = request.getStartAfter_asU8();

    let startAfter;
    if (startAfterBinary.length > 0) {
      startAfter = Buffer.from(startAfterBinary);
    }

    // Start at

    const startAtBinary = request.getStartAt_asU8();

    let startAt;
    if (startAtBinary.length > 0) {
      startAt = Buffer.from(startAtBinary);
    }

    const options = {
      where,
      orderBy,
      limit,
      startAfter,
      startAt,
    };

    // Prove

    const prove = request.getProve();

    const documentResponseBuffer = await driveClient.fetchDocuments(
      Buffer.from(dataContractId), documentType, options, prove,
    );

    return GetDocumentsResponse.deserializeBinary(documentResponseBuffer);
  }

  return getDocumentsHandler;
}

module.exports = getDocumentsHandlerFactory;

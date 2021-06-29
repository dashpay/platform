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

const AbciResponseError = require('../../../errors/AbciResponseError');

/**
 *
 * @param {DriveClient} driveClient
 * @param {handleAbciResponseError} handleAbciResponseError
 *
 * @returns {getDocumentsHandler}
 */
function getDocumentsHandlerFactory(driveClient, handleAbciResponseError) {
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

    const whereBinary = request.getWhere();

    let where;
    if (whereBinary && whereBinary.length > 0) {
      where = cbor.decode(
        Buffer.from(whereBinary),
      );
    }

    // Order by

    const orderByBinary = request.getOrderBy();

    let orderBy;
    if (orderByBinary && orderByBinary.length > 0) {
      orderBy = cbor.decode(
        Buffer.from(orderByBinary),
      );
    }

    // Limit

    const limitDefault = request.getLimit();

    let limit;
    if (limitDefault !== 0) {
      limit = limitDefault;
    }

    // Start after

    const startAfterDefault = request.getStartAfter();

    let startAfter;
    if (startAfterDefault !== 0) {
      startAfter = startAfterDefault;
    }

    // Start at

    const startAtDefault = request.getStartAt();

    let startAt;
    if (startAtDefault) {
      startAt = startAtDefault;
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

    let documentResponseBuffer;
    try {
      documentResponseBuffer = await driveClient.fetchDocuments(
        dataContractId, documentType, options, prove,
      );
    } catch (e) {
      if (e instanceof AbciResponseError) {
        handleAbciResponseError(e);
      }
      throw e;
    }

    return GetDocumentsResponse.deserializeBinary(documentResponseBuffer);
  }

  return getDocumentsHandler;
}

module.exports = getDocumentsHandlerFactory;

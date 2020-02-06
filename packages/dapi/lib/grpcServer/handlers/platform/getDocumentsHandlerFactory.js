const cbor = require('cbor');

const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  GetDocumentsResponse,
} = require('@dashevo/dapi-grpc');

const RPCError = require('../../../rpcServer/RPCError');

/**
 *
 * @param {DriveAdapter} driveAPI
 * @param {DashPlatformProtocol} dpp
 * @returns {getDocumentsHandler}
 */
function getDocumentsHandlerFactory(driveAPI, dpp) {
  /**
   * @typedef getDocumentsHandler
   * @param {Object} call
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
    if (whereBinary) {
      where = cbor.decode(
        Buffer.from(whereBinary),
      );
    }

    // Order by

    const orderByBinary = request.getOrderBy();

    let orderBy;
    if (orderByBinary) {
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

    let documentsJSON;
    try {
      documentsJSON = await driveAPI.fetchDocuments(dataContractId, documentType, options);
    } catch (e) {
      if (e instanceof RPCError && e.code === -32602) {
        throw new InvalidArgumentGrpcError(e.message, e.data);
      }

      throw e;
    }

    const documents = await Promise.all(
      documentsJSON.map(documentJSON => dpp.document.createFromObject(
        documentJSON,
        { skipValidation: true },
      )),
    );

    const response = new GetDocumentsResponse();
    response.setDocumentsList(documents.map(document => document.serialize()));

    return response;
  }

  return getDocumentsHandler;
}

module.exports = getDocumentsHandlerFactory;

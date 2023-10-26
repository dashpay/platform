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

    if (!request.getV0().getDataContractId()) {
      throw new InvalidArgumentGrpcError('dataContractId is not specified');
    }

    if (!request.getV0().getDocumentType()) {
      throw new InvalidArgumentGrpcError('documentType is not specified');
    }

    const documentResponseBuffer = await driveClient.fetchDocuments(
      request,
    );

    return GetDocumentsResponse.deserializeBinary(documentResponseBuffer);
  }

  return getDocumentsHandler;
}

module.exports = getDocumentsHandlerFactory;

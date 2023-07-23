/**
 * @param {DriveClient} driveClient
 *
 * @return {getDataContractsHandler}
 */
function getDataContractsHandlerFactory(driveClient) {
  /**
   * @typedef getDataContractsHandler
   *
   * @param {Object} call
   *
   * @returns {Promise<GetDataContractsResponse}
   */
  async function getDataContractsHandler(call) {
    const { request } = call;

    const dataContractIds = request.getIds();

    if (dataContractIds === null) {
      throw new InvalidArgumentGrpcError('data contract ids are not specified');
    }

    const dataContractsResponseBuffer = await driveClient.fetchDataContracts(request);

    return GetDataContractsResponse.deserializeBinary(dataContractsResponseBuffer);
  }

  return getDataContractsHandler;
}

module.exports = getDataContractsHandlerFactory;

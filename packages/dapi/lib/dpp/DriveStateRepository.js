const {
  v0: {
    GetDataContractResponse,
  },
} = require('@dashevo/dapi-grpc');

/**
 * @implements StateRepository
 */
class DriveStateRepository {
  /**
   * @param {DriveClient} driveClient
   * @param {DashPlatformProtocol} dpp
   */
  constructor(driveClient, dpp) {
    this.driveClient = driveClient;
    this.dpp = dpp;
  }

  /**
   * Fetches data contract from Drive
   * @param {Identifier} contractIdentifier
   * @return {Promise<DataContract>}
   */
  async fetchDataContract(contractIdentifier) {
    const dataContractProtoBuffer = await this.driveClient.fetchDataContract(
      contractIdentifier, false,
    );

    const dataContractResponse = GetDataContractResponse.deserializeBinary(
      dataContractProtoBuffer,
    );

    return this.dpp.dataContract.createFromBuffer(
      Buffer.from(dataContractResponse.getDataContract()),
    );
  }
}

module.exports = DriveStateRepository;

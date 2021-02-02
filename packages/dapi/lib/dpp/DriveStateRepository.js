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
    const driveResponse = await this.driveClient.fetchDataContract(
      contractIdentifier, false,
    );

    return this.dpp.dataContract.createFromBuffer(driveResponse.data);
  }
}

module.exports = DriveStateRepository;

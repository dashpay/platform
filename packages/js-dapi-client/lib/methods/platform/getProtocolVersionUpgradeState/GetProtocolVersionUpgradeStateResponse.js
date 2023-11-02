const AbstractResponse = require('../response/AbstractResponse');
const VersionEntry = require('./VersionEntry');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

class GetProtocolVersionUpgradeStateResponse extends AbstractResponse {
  /**
   * @param {VersionEntry[]} versionEntries
   * @param {Metadata} metadata
   * @param {Proof} [proof]
   */
  constructor(versionEntries, metadata, proof = undefined) {
    super(metadata, proof);

    this.versionEntries = versionEntries;
  }

  /**
   * @returns {VersionEntry[]}
   */
  getVersionEntries() {
    return this.versionEntries;
  }

  /**
   * @param proto
   * @returns {GetProtocolVersionUpgradeStateResponse}
   */
  static createFromProto(proto) {
    const versions = proto.getV0().getVersions();
    const { metadata, proof } = AbstractResponse.createMetadataAndProofFromProto(
      proto,
    );

    if (!versions && !proof) {
      throw new InvalidResponseError('Version upgrade state is not defined');
    }

    let versionEntries = [];

    const versionsList = versions && versions.getVersionsList();
    if (versionsList) {
      versionEntries = versionsList.map((versionSignal) => new VersionEntry(
        versionSignal.getVersionNumber(),
        versionSignal.getVoteCount(),
      ));
    }

    return new GetProtocolVersionUpgradeStateResponse(
      versionEntries,
      metadata,
      proof,
    );
  }
}

module.exports = GetProtocolVersionUpgradeStateResponse;

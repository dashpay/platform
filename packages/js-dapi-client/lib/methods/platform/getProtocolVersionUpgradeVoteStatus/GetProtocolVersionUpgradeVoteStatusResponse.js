const AbstractResponse = require('../response/AbstractResponse');
const VersionSignal = require('./VersionSignal');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

class GetProtocolVersionUpgradeVoteStatusResponse extends AbstractResponse {
  /**
   * @param {VersionSignal[]} versionSignals
   * @param {Metadata} metadata
   * @param {Proof} [proof]
   */
  constructor(versionSignals, metadata, proof = undefined) {
    super(metadata, proof);

    this.versionSignals = versionSignals;
  }

  /**
   * @returns {VersionSignal[]}
   */
  getVersionSignals() {
    return this.versionSignals;
  }

  /**
   * @param proto
   * @returns {GetProtocolVersionUpgradeVoteStatusResponse}
   */
  static createFromProto(proto) {
    const versions = proto.getV0().getVersions();
    const { metadata, proof } = AbstractResponse.createMetadataAndProofFromProto(
      proto,
    );

    if (!versions && !proof) {
      throw new InvalidResponseError('Version upgrade vote status is not defined');
    }

    let versionSignals = [];

    const versionSignalsList = versions && versions.getVersionSignalsList();
    if (versionSignalsList) {
      versionSignals = versionSignalsList.map((versionSignal) => new VersionSignal(
        Buffer.from(versionSignal.getProTxHash()).toString('hex'),
        versionSignal.getVersion(),
      ));
    }

    return new GetProtocolVersionUpgradeVoteStatusResponse(
      versionSignals,
      metadata,
      proof,
    );
  }
}

module.exports = GetProtocolVersionUpgradeVoteStatusResponse;

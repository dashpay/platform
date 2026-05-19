import AbstractResponse from '../response/AbstractResponse.js';
import VersionSignal from './VersionSignal.js';
import InvalidResponseError from '../response/errors/InvalidResponseError.js';
import { bytesToHex } from '../../../utils/bytes.js';

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
      throw new InvalidResponseError('Version upgrade votes status is not defined');
    }

    let versionSignals = [];

    const versionSignalsList = versions && versions.getVersionSignalsList();
    if (versionSignalsList) {
      versionSignals = versionSignalsList.map((versionSignal) => new VersionSignal(
        bytesToHex(new Uint8Array(versionSignal.getProTxHash())),
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

export default GetProtocolVersionUpgradeVoteStatusResponse;

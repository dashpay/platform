const AbstractResponse = require('../response/AbstractResponse');
const EpochInfo = require('./EpochInfo');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

class GetEpochsInfoResponse extends AbstractResponse {
  /**
   * @param {EpochInfo[]} epochsInfo
   * @param {Metadata} metadata
   * @param {Proof} [proof]
   */
  constructor(epochsInfo, metadata, proof = undefined) {
    super(metadata, proof);

    this.epochsInfo = epochsInfo;
  }

  /**
   * @returns {EpochInfo[]}
   */
  getEpochsInfo() {
    return this.epochsInfo;
  }

  /**
   * @param proto
   * @returns {GetEpochsInfoResponse}
   */
  static createFromProto(proto) {
    const epochs = proto.getV0().getEpochs();
    const { metadata, proof } = AbstractResponse.createMetadataAndProofFromProto(
      proto,
    );

    if (!epochs && !proof) {
      throw new InvalidResponseError('Epochs are not defined');
    }

    let epochsInfo = [];

    const epochsInfoList = epochs && epochs.getEpochInfosList();
    if (epochsInfoList) {
      epochsInfo = epochsInfoList.map((epoch) => new EpochInfo(
        epoch.getNumber(),
        epoch.getFirstBlockHeight(),
        epoch.getFirstCoreBlockHeight(),
        epoch.getStartTime(),
        epoch.getFeeMultiplier(),
      ));
    }

    return new GetEpochsInfoResponse(
      epochsInfo,
      metadata,
      proof,
    );
  }
}

module.exports = GetEpochsInfoResponse;

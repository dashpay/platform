const AbstractResponse = require('../response/AbstractResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

class GetDataContractHistoryResponse extends AbstractResponse {
  /**
   * @param {Buffer[]} dataContractHistory
   * @param {Metadata} metadata
   * @param {Proof} [proof]
   */
  constructor(dataContractHistory, metadata, proof = undefined) {
    super(metadata, proof);

    this.dataContractHistory = dataContractHistory;
  }

  /**
   * @returns {Buffer}
   */
  getDataContract() {
    return this.dataContract;
  }

  /**
   * @param proto
   * @returns {GetDataContractHistoryResponse}
   */
  static createFromProto(proto) {
    // TODO: is that an array of buffers?
    const dataContractHistory = proto.getDataContractHistory();
    const { metadata, proof } = AbstractResponse.createMetadataAndProofFromProto(proto);

    if (!dataContractHistory && !proof) {
      throw new InvalidResponseError('DataContract is not defined');
    }

    // TODO: stopped here
    dataContractHistory.map(historyEntry => {
        if (!historyEntry.getValue()) {
            throw new InvalidResponseError('DataContract is not defined');
        }
    })

    return new GetDataContractHistoryResponse(
      Buffer.from(dataContractHistory),
      metadata,
      proof,
    );
  }
}

module.exports = GetDataContractHistoryResponse;

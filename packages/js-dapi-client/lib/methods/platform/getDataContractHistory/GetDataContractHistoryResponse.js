const AbstractResponse = require('../response/AbstractResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

class GetDataContractHistoryResponse extends AbstractResponse {
  /**
   * @param {object.<number, Buffer>} dataContractHistory
   * @param {Metadata} metadata
   * @param {Proof} [proof]
   */
  constructor(dataContractHistory, metadata, proof = undefined) {
    super(metadata, proof);

    this.dataContractHistory = dataContractHistory;
  }

  /**
   * @returns {object.<number, Buffer>}
   */
  getDataContractHistory() {
    return this.dataContractHistory;
  }

  /**
   * @param proto
   * @returns {GetDataContractHistoryResponse}
   */
  static createFromProto(proto) {
    // History is something that we need to call a method to get a list of entries on
    const dataContractHistory = proto.getV0().getDataContractHistory();
    const { metadata, proof } = AbstractResponse.createMetadataAndProofFromProto(proto);

    if (!dataContractHistory && !proof) {
      throw new InvalidResponseError('DataContract is not defined');
    }

    const history = {};

    if (dataContractHistory) {
      const dataContractHistoryEntries = dataContractHistory.getDataContractEntriesList();

      // eslint-disable-next-line no-restricted-syntax
      for (const historyEntry of dataContractHistoryEntries) {
        history[historyEntry.getDate()] = historyEntry.getValue();
      }
    }

    return new GetDataContractHistoryResponse(
      history,
      metadata,
      proof,
    );
  }
}

module.exports = GetDataContractHistoryResponse;

import AbstractResponse from '../response/AbstractResponse.js';
import InvalidResponseError from '../response/errors/InvalidResponseError.js';
import DataContractHistoryEntry from './DataContractHistoryEntry.js';

class GetDataContractHistoryResponse extends AbstractResponse {
  /**
   * @param {DataContractHistoryEntry[]} dataContractHistory
   * @param {Metadata} metadata
   * @param {Proof} [proof]
   */
  constructor(dataContractHistory, metadata, proof = undefined) {
    super(metadata, proof);

    this.dataContractHistory = dataContractHistory;
  }

  /**
   * @returns {DataContractHistoryEntry[]} array of data contract history entries
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

    return new GetDataContractHistoryResponse(
      dataContractHistory ? dataContractHistory.getDataContractEntriesList()
        .map((dataContractHistoryEntry) => new DataContractHistoryEntry(
          BigInt(dataContractHistoryEntry.getDate()),
          dataContractHistoryEntry.getValue(),
        )) : null,
      metadata,
      proof,
    );
  }
}

export default GetDataContractHistoryResponse;

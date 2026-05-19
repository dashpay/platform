const AbstractResponse = require('../response/AbstractResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

class GetDataContractResponse extends AbstractResponse {
  /**
   * @param {Uint8Array} dataContract
   * @param {Metadata} metadata
   * @param {Proof} [proof]
   */
  constructor(dataContract, metadata, proof = undefined) {
    super(metadata, proof);

    this.dataContract = dataContract;
  }

  /**
   * @returns {Uint8Array}
   */
  getDataContract() {
    return this.dataContract;
  }

  /**
   * @param proto
   * @returns {GetDataContractResponse}
   */
  static createFromProto(proto) {
    const dataContract = proto.getV0().getDataContract();
    const { metadata, proof } = AbstractResponse.createMetadataAndProofFromProto(proto);

    if (!dataContract && !proof) {
      throw new InvalidResponseError('DataContract is not defined');
    }

    return new GetDataContractResponse(
      new Uint8Array(dataContract),
      metadata,
      proof,
    );
  }
}

module.exports = GetDataContractResponse;

const AbstractResponse = require('../response/AbstractResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

class GetDataContractResponse extends AbstractResponse {
  /**
   * @param {Buffer} dataContract
   * @param {Metadata} metadata
   * @param {Proof} [proof]
   */
  constructor(dataContract, metadata, proof = undefined) {
    super(metadata, proof);

    this.dataContract = dataContract;
  }

  /**
   * @returns {Buffer}
   */
  getDataContract() {
    return this.dataContract;
  }

  /**
   * @param proto
   * @returns {GetDataContractResponse}
   */
  static createFromProto(proto) {
    const dataContract = proto.getDataContract();
    const { metadata, proof } = AbstractResponse.createMetadataAndProofFromProto(proto);

    if (!dataContract && !proof) {
      throw new InvalidResponseError('DataContract is not defined');
    }

    return new GetDataContractResponse(
      Buffer.from(dataContract),
      metadata,
      proof,
    );
  }
}

module.exports = GetDataContractResponse;

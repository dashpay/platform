const AbstractResponse = require('../response/AbstractResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

class GetContestedResourcesResponse extends AbstractResponse {
  /**
   * @param {object} contestedResourceContenders
   * @param {Metadata} metadata
   * @param {Proof} [proof]
   */
  constructor(contestedResources, resultCase, metadata, proof = undefined) {
    super(metadata, proof);

    this.contestedResources = contestedResources;
    this.resultCase = resultCase;
  }

  /**
   * @returns {object}
   */
  getContestedResources() {
    return this.contestedResources;
  }

  /**
   * @param proto
   * @returns {GetContestedResourceResponse}
   */
  static createFromProto(proto) {
    // eslint-disable-next-line
    const contestedResourceContenders = proto.getV0().getContestedResourceValues();
    const resultCase = proto.getV0().getResultCase()

    const { metadata, proof } = AbstractResponse.createMetadataAndProofFromProto(
      proto,
    );

    if ((typeof contestedResourceContenders === 'undefined' || contestedResourceContenders === null) && !proof) {
      throw new InvalidResponseError('Contested Resource Contenders data is not defined');
    }

    return new GetContestedResourcesResponse(
      contestedResourceContenders.toObject(),
      resultCase,
      metadata,
      proof,
    );
  }
}

module.exports = GetContestedResourcesResponse;

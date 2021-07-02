const AbstractResponse = require('../response/AbstractResponse');
const Metadata = require('../response/Metadata');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

class GetIdentityResponse extends AbstractResponse {
  /**
   * @param {Buffer} identity
   * @param {Metadata} metadata
   */
  constructor(identity, metadata) {
    super(metadata);

    this.identity = identity;
  }

  /**
   * @returns {Buffer}
   */
  getIdentity() {
    return this.identity;
  }

  /**
   * @param proto
   * @returns {GetIdentityResponse}
   */
  static createFromProto(proto) {
    const identity = proto.getIdentity();

    if (!identity) {
      throw new InvalidResponseError('Identity is not defined');
    }

    const metadata = proto.getMetadata();

    if (metadata === undefined) {
      throw new InvalidResponseError('Metadata is not defined');
    }

    return new GetIdentityResponse(
      Buffer.from(proto.getIdentity()),
      new Metadata(metadata.toObject()),
    );
  }
}

module.exports = GetIdentityResponse;

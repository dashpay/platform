const AbstractResponse = require('../response/AbstractResponse');
const Metadata = require('../response/Metadata');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

class GetIdentityIdsByPublicKeyHashesResponse extends AbstractResponse {
  /**
   * @param {Buffer[]} identityIds
   * @param {Metadata} metadata
   */
  constructor(identityIds, metadata) {
    super(metadata);

    this.identityIds = identityIds;
  }

  /**
   * @returns {Buffer[]}
   */
  getIdentityIds() {
    return this.identityIds;
  }

  /**
   * @param proto
   * @returns {GetIdentityIdsByPublicKeyHashesResponse}
   */
  static createFromProto(proto) {
    const metadata = proto.getMetadata();

    if (metadata === undefined) {
      throw new InvalidResponseError('Metadata is not defined');
    }

    return new GetIdentityIdsByPublicKeyHashesResponse(
      proto.getIdentityIdsList()
        .map((identityId) => (identityId.length > 0 ? Buffer.from(identityId) : null)),
      new Metadata(metadata.toObject()),
    );
  }
}

module.exports = GetIdentityIdsByPublicKeyHashesResponse;

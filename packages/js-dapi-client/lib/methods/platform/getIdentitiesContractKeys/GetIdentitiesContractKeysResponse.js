const { Identifier } = require('@dashevo/wasm-dpp');
const AbstractResponse = require('../response/AbstractResponse');

class GetIdentitiesContractKeysResponse extends AbstractResponse {
  /**
   * @param {object} identitiesKeys
   * @param {Metadata} metadata
   * @param {Proof} [proof]
   */
  constructor(identitiesKeys, metadata, proof = undefined) {
    super(metadata, proof);

    this.identitiesKeys = identitiesKeys;
  }

  /**
   * @returns {object}
   */
  getIdentitiesKeys() {
    return this.identitiesKeys;
  }

  /**
   * @param proto
   * @returns {GetIdentitiesContractKeysResponse}
   */
  static createFromProto(proto) {
    const { metadata, proof } = AbstractResponse.createMetadataAndProofFromProto(proto);

    const identitiesKeys = proto.getV0().getIdentitiesKeys();

    let identitiesKeysMap = {};
    if (identitiesKeys) {
      const keysEntries = identitiesKeys.getEntriesList();

      identitiesKeysMap = keysEntries.reduce((acc, entry) => {
        const identityId = Identifier.from(Buffer.from(entry.getIdentityId())).toString();
        if (!acc[identityId]) {
          acc[identityId] = {};
        }

        entry.getKeysList().forEach((key) => {
          const purpose = key.getPurpose();
          if (!acc[identityId][purpose]) {
            // eslint-disable-next-line no-param-reassign
            acc[identityId][purpose] = [];
          }

          // eslint-disable-next-line no-param-reassign
          acc[identityId][purpose] = acc[identityId][purpose].concat(key.getKeysBytesList());
        }, {});

        return acc;
      }, {});
    }

    return new GetIdentitiesContractKeysResponse(
      identitiesKeysMap,
      metadata,
      proof,
    );
  }
}

module.exports = GetIdentitiesContractKeysResponse;

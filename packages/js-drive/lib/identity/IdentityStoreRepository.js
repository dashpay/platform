const Identity = require('@dashevo/dpp/lib/identity/Identity');

const Read = require('@dashevo/dpp/lib/stateTransition/fees/operations/ReadOperation');
const Write = require('@dashevo/dpp/lib/stateTransition/fees/operations/WriteOperation');

class IdentityStoreRepository {
  /**
   *
   * @param {GroveDBStore} groveDBStore
   * @param {decodeProtocolEntity} decodeProtocolEntity
   */
  constructor(groveDBStore, decodeProtocolEntity) {
    this.storage = groveDBStore;
    this.decodeProtocolEntity = decodeProtocolEntity;
  }

  /**
   * Store identity into database
   *
   * @param {Identity} identity
   * @param {boolean} [useTransaction=false]
   * @return {Promise<IdentityStoreRepository>}
   */
  async store(identity, useTransaction = false) {
    await this.storage.put(
      IdentityStoreRepository.TREE_PATH,
      identity.getId().toBuffer(),
      identity.toBuffer(),
      { useTransaction },
    );

    return {
      result: this,
      operations: [
        new Write(
          identity.getId().length,
          identity.toBuffer().length,
        ),
      ],
    };
  }

  /**
   * Fetch identity by id from database
   *
   * @param {Identifier} id
   * @param {boolean} [useTransaction=false]
   * @return {Promise<null|Identity>}
   */
  async fetch(id, useTransaction = false) {
    const encodedIdentity = await this.storage.get(
      IdentityStoreRepository.TREE_PATH,
      id.toBuffer(),
      { useTransaction },
    );

    if (!encodedIdentity) {
      return {
        result: null,
        operations: [
          new Read(
            id.length, 
            IdentityStoreRepository.TREE_PATH.reduce((size, pathItem) => size + pathItem.length, 0),
            0,
          )
        ],
      };
    }

    const [protocolVersion, rawIdentity] = this.decodeProtocolEntity(encodedIdentity);

    rawIdentity.protocolVersion = protocolVersion;

    return {
      result: new Identity(rawIdentity),
      operations: [
        new Read(
          id.length, 
          IdentityStoreRepository.TREE_PATH.reduce((size, pathItem) => size + pathItem.length, 0),
          encodedIdentity.length,
        )
      ],
    };
  }

  /**
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.skipIfExists]
   *
   * @return {Promise<IdentityStoreRepository>}
   */
  async createTree(options = {}) {
    await this.storage.createTree([], IdentityStoreRepository.TREE_PATH[0], options);

    return {
      result: this,
      operations: [
        new Write(
          IdentityStoreRepository.TREE_PATH[0].length, 
          32,
        ),
      ],
    };
  }
}

IdentityStoreRepository.TREE_PATH = [Buffer.from([0])];

module.exports = IdentityStoreRepository;

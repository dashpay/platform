const Document = require('@dashevo/dpp/lib/document/Document');

const decodeProtocolEntityFactory = require('@dashevo/dpp/lib/decodeProtocolEntityFactory');

const decodeProtocolEntity = decodeProtocolEntityFactory();

class DocumentStoreRepository {
  /**
   *
   * @param {GroveDBStore} groveDBStore
   */
  constructor(groveDBStore) {
    this.storage = groveDBStore;
  }

  /**
   * Store document
   *
   * @param {Document} document
   * @param {GroveDBTransaction} [transaction]
   * @return {Promise<IdentityStoreRepository>}
   */
  async store(document, transaction = undefined) {
    this.storage.put(
      document.getId().toBuffer(),
      document.toBuffer(),
      transaction,
    );
  }

  /**
   * Fetch document by id
   *
   * @param {Identifier} id
   * @param {GroveDBTransaction} [transaction]
   * @return {Promise<null|Document>}
   */
  async fetch(id, transaction = undefined) {
    const encodedDocument = this.storage.get(id.toBuffer(), transaction);

    if (!encodedDocument) {
      return null;
    }

    const [, rawDocument] = decodeProtocolEntity(encodedDocument);

    return new Document(rawDocument);
  }

  /**
   *
   * @param {Identifier} id
   * @param {GroveDBTransaction} transaction
   * @return {Promise<void>}
   */
  async delete(id, transaction = undefined) {
    this.storage.delete(
      id.toBuffer(),
      transaction,
    );
  }
}

module.exports = DocumentStoreRepository;

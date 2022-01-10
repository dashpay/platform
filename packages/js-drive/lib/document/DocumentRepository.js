const Document = require('@dashevo/dpp/lib/document/Document');

const lodashGet = require('lodash.get');

const decodeProtocolEntityFactory = require('@dashevo/dpp/lib/decodeProtocolEntityFactory');
const DataContractStoreRepository = require("../dataContract/DataContractStoreRepository");
const getPropertyDefinitionByPath = require("@dashevo/dpp/lib/dataContract/getPropertyDefinitionByPath");

const decodeProtocolEntity = decodeProtocolEntityFactory();

class DocumentRepository {
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
   * @param {DataContract} document
   * @param {Document} document
   * @param {GroveDBTransaction} [transaction]
   * @return {Promise<IdentityStoreRepository>}
   */
  async store(document, transaction = undefined) {
    const documentTypeTreePath = this.#getDocumentTypeTreePath(
      document.getDataContract(),
      documentType,
    );

    // Store document
    const isDocumentExist = Boolean(await this.storage.get(
      documentTypeTreePath.concat([ DataContractStoreRepository.ID_TREE_KEY]),
      document.getId().toBuffer(),
      { transaction },
    ));

    // TODO: Implement proper update
    if (isDocumentExist) {
      await this.delete(document.getId(), transaction);
    }

    // Store document
    await this.storage.put(
      documentTypeTreePath.concat([ DataContractStoreRepository.ID_TREE_KEY]),
      document.getId().toBuffer(),
      document.toBuffer(),
      { transaction },
    );

    // Create indexed property trees
    const rawDocument = document.toObject();

    const documentDefinition = document.getDataContract().getDocumentSchema(document.getType());

    const documentIndices = documentDefinition.indices || [];

    await Promise.all(documentIndices.map(async (indexDefinition) => {
      let indexedPropertiesPath = documentTypeTreePath;

      return Promise.all(indexDefinition.properties.map(async (propertyAndOrder, i) => {
        const propertyName = Object.keys(propertyAndOrder)[0];

        // const propertyDefinition = getPropertyDefinitionByPath(documentDefinition, propertyName);

        const propertyValue = lodashGet(rawDocument, propertyName);

        if (propertyValue === undefined) {
          return;
        }

        // Create tree for indexed property
        await this.storage.createTree(
          indexedPropertiesPath,
          Buffer.from(propertyName),
          { transaction, skipIfExists: true },
        );

        // Create a value subtree
        await this.storage.createTree(
          indexedPropertiesPath.concat([Buffer.from(propertyName)]),
          Buffer.from(propertyValue), // TODO: Encode value depending on the type
          { transaction, skipIfExists: true },
        );

        indexedPropertiesPath = indexedPropertiesPath.concat([Buffer.from(propertyName), Buffer.from(propertyValue)]);

        // Create tree for ID references
        if (i === indexDefinition.properties.length - 1) {
          await this.storage.createTree(
            indexedPropertiesPath,
            DataContractStoreRepository.ID_TREE_KEY, // TODO: Encode value depending on the type
            {
              transaction,
              skipIfExists: true
            },
          );

          // Create tree for ID references
          await this.storage.put(
            indexedPropertiesPath.concat([DataContractStoreRepository.ID_TREE_KEY]),
            document.getId().toBuffer(),
            document.toBuffer(), // TODO: must be reference
            {
              transaction,
              skipIfExists: true
            },
          );
        }
      }));
    }));
  }

  /**
   * Fetch document by id
   *
   * @param {DataContract} dataContract
   * @param {string} documentType
   * @param {Identifier} id
   * @param {GroveDBTransaction} [transaction]
   * @return {Promise<null|Document>}
   */
  async fetch(dataContract, documentType, id, transaction = undefined) {
    const documentTypeTreePath = this.#getDocumentTypeTreePath(dataContract, documentType);

    // Store document
    const encodedDocument = await this.storage.get(
      documentTypeTreePath.concat([ DataContractStoreRepository.ID_TREE_KEY]),
      id.toBuffer(),
      { transaction },
    );

    if (!encodedDocument) {
      return null;
    }

    const [, rawDocument] = decodeProtocolEntity(encodedDocument);

    return new Document(rawDocument, dataContract);
  }

  /**
   *
   * @param {DataContract} dataContract
   * @param {string} documentType
   * @param {Identifier} id
   * @param {GroveDBTransaction} transaction
   * @return {Promise<void>}
   */
  async delete(dataContract, documentType, id, transaction = undefined) {
    const documentTypeTreePath = this.#getDocumentTypeTreePath(
      dataContract,
      documentType,
    );

    // Delete document
    await this.storage.delete(
      documentTypeTreePath.concat([DataContractStoreRepository.ID_TREE_KEY]),
      id.toBuffer(),
      { transaction },
    );
  }

  /**
   * @param {DataContract} dataContract
   * @param {string} documentType
   * @return {Buffer[]}
   */
  #getDocumentTypeTreePath(dataContract, documentType) {
    return DataContractStoreRepository.TREE_PATH.concat([
      document.getDataContractId().toBuffer(),
      document.getType()
    ]);
  }
}

module.exports = DocumentRepository;

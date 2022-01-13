const Document = require('@dashevo/dpp/lib/document/Document');

const lodashGet = require('lodash.get');

const decodeProtocolEntityFactory = require('@dashevo/dpp/lib/decodeProtocolEntityFactory');
const DataContractStoreRepository = require("../../dataContract/DataContractStoreRepository");
const getPropertyDefinitionByPath = require("@dashevo/dpp/lib/dataContract/getPropertyDefinitionByPath");

const decodeProtocolEntity = decodeProtocolEntityFactory();

class DocumentRepository {
  /**
   *
   * @param {GroveDBStore} groveDBStore
   * @param {encodeInteger} encodeInteger
   * @param {encodeFloat} encodeFloat
   */
  constructor(groveDBStore, encodeDocumentPropertyValue) {
    this.storage = groveDBStore;
    this.encodeDocumentPropertyValue = encodeDocumentPropertyValue;
    this.encodeFloat = encodeFloat;
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
      document.getType(),
    );

    const documentIdsTreePath = documentTypeTreePath.concat([ DataContractStoreRepository.DOCUMENTS_TREE_KEY]);

    const isDocumentAlreadyExist = Boolean(await this.storage.get(
      documentIdsTreePath,
      document.getId().toBuffer(),
      { transaction },
    ));

    // TODO: Implement proper update
    if (isDocumentAlreadyExist) {
      await this.delete(
        document.getDataContract(),
        document.getType(),
        document.getId(),
        transaction,
      );
    }

    // Store document
    await this.storage.put(
      documentIdsTreePath,
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

        const propertyDefinition = getPropertyDefinitionByPath(documentDefinition, propertyName);

        const propertyValue = lodashGet(rawDocument, propertyName);

        if (propertyValue === undefined) {
          return;
        }

        // Create tree for indexed property if not exists
        await this.storage.createTree(
          indexedPropertiesPath,
          Buffer.from(propertyName),
          { transaction, skipIfExists: true },
        );

        // Create a value subtree if not exists
        const propertyTreePath = indexedPropertiesPath.concat([Buffer.from(propertyName)]);

        const encodedPropertyValue = this.encodeDocumentPropertyValue(propertyValue, propertyDefinition);

        await this.storage.createTree(
          propertyTreePath,
          encodedPropertyValue,
          { transaction, skipIfExists: true },
        );

        indexedPropertiesPath = propertyTreePath.concat([encodedPropertyValue]);

        // Create tree for ID references if not exists
        if (i === indexDefinition.properties.length - 1) {
          await this.storage.createTree(
            indexedPropertiesPath,
            DataContractStoreRepository.DOCUMENTS_TREE_KEY,
            {
              transaction,
              skipIfExists: true
            },
          );

          const documentPath = DataContractStoreRepository.TREE_PATH.concat([
            document.getDataContractId().toBuffer(),
            Buffer.from(document.getType()),

          ]);

          // Store
          await this.storage.putReference(
            indexedPropertiesPath.concat([DataContractStoreRepository.DOCUMENTS_TREE_KEY]),
            document.getId().toBuffer(),
            documentPath,
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
      documentTypeTreePath.concat([ DataContractStoreRepository.DOCUMENTS_TREE_KEY]),
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
      documentTypeTreePath.concat([DataContractStoreRepository.DOCUMENTS_TREE_KEY]),
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

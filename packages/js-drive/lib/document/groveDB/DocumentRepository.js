const Document = require('@dashevo/dpp/lib/document/Document');

const getPropertyDefinitionByPath = require('@dashevo/dpp/lib/dataContract/getPropertyDefinitionByPath');

const DataContractStoreRepository = require('../../dataContract/DataContractStoreRepository');
const InvalidQueryError = require('../errors/InvalidQueryError');
const createDocumentTypeTreePath = require('./createDocumentTreePath');

class DocumentRepository {
  /**
   *
   * @param {GroveDBStore} groveDBStore
   * @param {encodeDocumentPropertyValue} encodeDocumentPropertyValue
   * @param {validateQuery} validateQuery
   * @param {decodeProtocolEntity} decodeProtocolEntity
   * @param {createGroveDBPathQuery} createGroveDBPathQuery
   */
  constructor(
    groveDBStore,
    encodeDocumentPropertyValue,
    validateQuery,
    decodeProtocolEntity,
    createGroveDBPathQuery,
  ) {
    this.storage = groveDBStore;
    this.encodeDocumentPropertyValue = encodeDocumentPropertyValue;
    this.validateQuery = validateQuery;
    this.decodeProtocolEntity = decodeProtocolEntity;
    this.createGroveDBPathQuery = createGroveDBPathQuery;
  }

  /**
   * Store document
   *
   * @param {DataContract} document
   * @param {Document} document
   * @param {boolean} [useTransaction=false]
   * @return {Promise<IdentityStoreRepository>}
   */
  async store(document, useTransaction = false) {
    const documentTypeTreePath = createDocumentTypeTreePath(
      document.getDataContract(),
      document.getType(),
    );

    const documentIdsTreePath = documentTypeTreePath.concat([
      DataContractStoreRepository.DOCUMENTS_TREE_KEY,
    ]);

    const isDocumentAlreadyExist = Boolean(await this.storage.get(
      documentIdsTreePath,
      document.getId().toBuffer(),
      { useTransaction },
    ));

    // TODO: Implement proper update
    if (isDocumentAlreadyExist) {
      await this.delete(
        document.getDataContract(),
        document.getType(),
        document.getId(),
        useTransaction,
      );
    }

    // Store document
    await this.storage.put(
      documentIdsTreePath,
      document.getId().toBuffer(),
      document.toBuffer(),
      { useTransaction },
    );

    // Create indexed property trees
    const documentDefinition = document.getDataContract().getDocumentSchema(document.getType());

    const documentIndices = documentDefinition.indices || [];

    await Promise.all(documentIndices.map(async (indexDefinition) => {
      let indexedPropertiesPath = documentTypeTreePath;

      return Promise.all(indexDefinition.properties.map(async (propertyAndOrder, i) => {
        const propertyName = Object.keys(propertyAndOrder)[0];

        const propertyValue = document.get(propertyName);

        if (propertyValue === undefined) {
          return;
        }

        // Create tree for indexed property if not exists
        await this.storage.createTree(
          indexedPropertiesPath,
          Buffer.from(propertyName),
          { useTransaction, skipIfExists: true },
        );

        // Create a value subtree if not exists
        const propertyTreePath = indexedPropertiesPath.concat([Buffer.from(propertyName)]);

        const propertyDefinition = getPropertyDefinitionByPath(documentDefinition, propertyName);

        // TODO: We need to apply sorting order defined in index to speed up
        const encodedPropertyValue = this.encodeDocumentPropertyValue(
          propertyValue,
          propertyDefinition,
        );

        await this.storage.createTree(
          propertyTreePath,
          encodedPropertyValue,
          { useTransaction, skipIfExists: true },
        );

        indexedPropertiesPath = propertyTreePath.concat([encodedPropertyValue]);

        // Create tree for ID references if not exists
        if (i === indexDefinition.properties.length - 1) {
          await this.storage.createTree(
            indexedPropertiesPath,
            DataContractStoreRepository.DOCUMENTS_TREE_KEY,
            {
              useTransaction,
              skipIfExists: true,
            },
          );

          const documentPath = documentIdsTreePath.concat([
            document.getId().toBuffer(),
          ]);

          // Store
          await this.storage.putReference(
            indexedPropertiesPath.concat([DataContractStoreRepository.DOCUMENTS_TREE_KEY]),
            document.getId().toBuffer(),
            documentPath,
            {
              useTransaction,
              skipIfExists: true,
            },
          );
        }
      }));
    }));
  }

  /**
   * Find documents with query
   *
   * @param dataContract
   * @param documentType
   * @param [query]
   * @param [query.where]
   * @param [query.limit]
   * @param [query.startAt]
   * @param [query.startAfter]
   * @param [query.orderBy]
   * @param {boolean} [useTransaction=false]
   *
   * @throws InvalidQueryError
   *
   * @returns {Document[]}
   */
  async find(dataContract, documentType, query = {}, useTransaction = false) {
    const documentSchema = dataContract.getDocumentSchema(documentType);

    const result = this.validateQuery(query, documentSchema);

    if (!result.isValid()) {
      throw new InvalidQueryError(result.getErrors());
    }

    const pathQuery = this.createGroveDBPathQuery(dataContract, documentType, query);

    const encodedDocuments = await this.storage.getWithQuery(pathQuery, useTransaction);

    return Promise.all(encodedDocuments.map(async (encodedDocument) => {
      const [, rawDocument] = this.decodeProtocolEntity(encodedDocument);

      return new Document(rawDocument, dataContract);
    }));
  }

  /**
   *
   * @param {DataContract} dataContract
   * @param {string} documentType
   * @param {Identifier} id
   * @param {boolean} useTransaction
   * @return {Promise<void>}
   */
  async delete(dataContract, documentType, id, useTransaction = false) {
    const documentTypeTreePath = createDocumentTypeTreePath(
      dataContract,
      documentType,
    );

    // Fetch document
    const encodedDocument = await this.storage.get(
      documentTypeTreePath.concat([DataContractStoreRepository.DOCUMENTS_TREE_KEY]),
      id.toBuffer(),
      { useTransaction },
    );

    if (!encodedDocument) {
      return;
    }

    /**
     * Remove index property subtrees
     */

    const [, rawDocument] = this.decodeProtocolEntity(encodedDocument);

    const document = new Document(rawDocument, dataContract);

    const documentDefinition = document.getDataContract().getDocumentSchema(document.getType());

    const documentIndices = documentDefinition.indices || [];

    await Promise.all(documentIndices.map(async (indexDefinition) => {
      let indexedPropertiesPath = documentTypeTreePath;

      return Promise.all(indexDefinition.properties.map(async (propertyAndOrder, i) => {
        const propertyName = Object.keys(propertyAndOrder)[0];

        const propertyValue = document.get(propertyName);

        if (propertyValue === undefined) {
          return;
        }

        const propertyDefinition = getPropertyDefinitionByPath(documentDefinition, propertyName);

        const encodedPropertyValue = this.encodeDocumentPropertyValue(
          propertyValue,
          propertyDefinition,
        );

        // Create a value subtree if not exists
        indexedPropertiesPath = indexedPropertiesPath.concat([
          Buffer.from(propertyName),
          encodedPropertyValue,
        ]);

        // TODO: We need to cleanup values too

        // Delete ID reference
        if (i === indexDefinition.properties.length - 1) {
          await this.storage.delete(
            indexedPropertiesPath.concat([DataContractStoreRepository.DOCUMENTS_TREE_KEY]),
            document.getId().toBuffer(),
            {
              useTransaction,
            },
          );
        }
      }));
    }));

    // Delete document
    await this.storage.delete(
      documentTypeTreePath.concat([DataContractStoreRepository.DOCUMENTS_TREE_KEY]),
      id.toBuffer(),
      { useTransaction },
    );
  }
}

module.exports = DocumentRepository;

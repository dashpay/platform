const { MongoClient } = require('mongodb');
const DashPlatformProtocol = require('@dashevo/dpp');

const wrapToErrorHandlerFactory = require('../../lib/api/jsonRpc/wrapToErrorHandlerFactory');

const SVContractMongoDbRepository = require('../stateView/contract/SVContractMongoDbRepository');

const fetchContractFactory = require('../stateView/contract/fetchContractFactory');
const fetchContractMethodFactory = require('../api/methods/fetchContractMethodFactory');

const SVDocumentMongoDbRepository = require('../stateView/document/mongoDbRepository/SVDocumentMongoDbRepository');
const createSVDocumentMongoDbRepositoryFactory = require('../stateView/document/mongoDbRepository/createSVDocumentMongoDbRepositoryFactory');
const convertWhereToMongoDbQuery = require('../stateView/document/mongoDbRepository/convertWhereToMongoDbQuery');
const validateQueryFactory = require('../stateView/document/query/validateQueryFactory');
const findConflictingConditions = require('../stateView/document/query/findConflictingConditions');
const fetchDocumentsFactory = require('../stateView/document/fetchDocumentsFactory');
const findNotIndexedFields = require('../stateView/document/query/findNotIndexedFields');
const findNotIndexedOrderByFields = require('../stateView/document/query/findNotIndexedOrderByFields');
const getIndexedFieldsFromDocumentSchema = require('../stateView/document/query/getIndexedFieldsFromDocumentSchema');
const fetchDocumentsMethodFactory = require('../api/methods/fetchDocumentsMethodFactory');

const checkReplicaSetInit = require('../mongoDb/checkReplicaSetInit');

const Logger = require('../../lib/util/Logger');

/**
 * Remove 'Method' Postfix
 *
 * Takes a function as an argument, returns the function's name
 * as a string without 'Method' as a postfix.
 *
 * @param {function} func Function that uses 'Method' postfix
 * @returns {string} String of function name without 'Method' postfix
 */
function rmPostfix(func) {
  const funcName = func.name;

  return funcName.substr(0, funcName.length - 'Method'.length);
}

class ApiApp {
  /**
   * @param {ApiAppOptions} options
   */
  constructor(options) {
    this.options = options;
    this.mongoClient = null;
  }

  /**
   * Init ApiApp
   */
  async init() {
    this.mongoClient = await MongoClient.connect(
      this.options.getStateViewMongoDBUrl(), {
        useNewUrlParser: true,
        useUnifiedTopology: true,
      },
    );

    const mongoDb = this.mongoClient.db(this.options.getStateViewMongoDBDatabase());

    await checkReplicaSetInit(mongoDb);
  }

  /**
   * @private
   * @return {Logger}
   */
  createLogger() {
    return new Logger(console);
  }

  /**
   * Create RPC methods with names
   *
   * @return {{string: Function}}
   */
  createRpcMethodsWithNames() {
    const methods = {};

    const logger = this.createLogger();
    const wrapToErrorHandler = wrapToErrorHandlerFactory(logger);

    for (const method of this.createRpcMethods()) {
      methods[rmPostfix(method)] = wrapToErrorHandler(method);
    }

    return methods;
  }

  /**
   * @private
   * @return {[ Function ]}
   */
  createRpcMethods() {
    return [
      this.createFetchContractMethod(),
      this.createFetchDocumentsMethod(),
    ];
  }

  /**
   * @private
   * @return {fetchContract}
   */
  createFetchContract() {
    if (!this.fetchContract) {
      const mongoDb = this.mongoClient.db(this.options.getStateViewMongoDBDatabase());
      const svContractMongoDbRepository = new SVContractMongoDbRepository(
        mongoDb,
        new DashPlatformProtocol(),
      );

      this.fetchContract = fetchContractFactory(svContractMongoDbRepository);
    }

    return this.fetchContract;
  }

  /**
   * @private
   * @returns {fetchContractMethod}
   */
  createFetchContractMethod() {
    return fetchContractMethodFactory(
      this.createFetchContract(),
    );
  }

  /**
   * @private
   * @return {fetchDocuments}
   */
  createFetchDocuments() {
    if (!this.fetchDocuments) {
      const validateQuery = validateQueryFactory(
        findConflictingConditions,
        getIndexedFieldsFromDocumentSchema,
        findNotIndexedFields,
        findNotIndexedOrderByFields,
      );

      const createSVDocumentMongoDbRepository = createSVDocumentMongoDbRepositoryFactory(
        this.mongoClient,
        SVDocumentMongoDbRepository,
        convertWhereToMongoDbQuery,
        validateQuery,
      );
      const mongoDb = this.mongoClient.db(this.options.getStateViewMongoDBDatabase());
      const svContractMongoDbRepository = new SVContractMongoDbRepository(
        mongoDb,
        new DashPlatformProtocol(),
      );
      this.fetchDocuments = fetchDocumentsFactory(
        createSVDocumentMongoDbRepository,
        svContractMongoDbRepository,
      );
    }

    return this.fetchDocuments;
  }

  /**
   * @private
   * @return {fetchDocumentsMethod}
   */
  createFetchDocumentsMethod() {
    return fetchDocumentsMethodFactory(
      this.createFetchDocuments(),
    );
  }
}

module.exports = ApiApp;

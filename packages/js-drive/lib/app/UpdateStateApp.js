const {
  StartTransactionRequest,
  ApplyStateTransitionRequest,
  CommitTransactionRequest,
  utils: {
    jsonToProtobufFactory,
    protobufToJsonFactory,
  },
  pbjs: {
    StartTransactionRequest: PBJSStartTransactionRequest,
    StartTransactionResponse: PBJSStartTransactionResponse,
    ApplyStateTransitionRequest: PBJSApplyStateTransitionRequest,
    ApplyStateTransitionResponse: PBJSApplyStateTransitionResponse,
    CommitTransactionRequest: PBJSCommitTransactionRequest,
    CommitTransactionResponse: PBJSCommitTransactionResponse,
  },
} = require('@dashevo/drive-grpc');

const RpcClient = require('@dashevo/dashd-rpc/promise');

const DashPlatformProtocol = require('@dashevo/dpp');
const { MongoClient } = require('mongodb');

const errorHandler = require('../util/errorHandler');
const jsonToProtobufHandlerWrapper = require('../grpcServer/jsonToProtobufHandlerWrapper');
const wrapInErrorHandlerFactory = require('../grpcServer/error/wrapInErrorHandlerFactory');
const startTransactionHandlerFactory = require('../grpcServer/handlers/startTransactionHandlerFactory');
const applyStateTransitionHandlerFactory = require('../grpcServer/handlers/applyStateTransitionHandlerFactory');
const commitTransactionHandlerFactory = require('../grpcServer/handlers/commitTransactionHandlerFactory');
const createContractDatabaseFactory = require('../stateView/contract/createContractDatabaseFactory');
const removeContractDatabaseFactory = require('../stateView/contract/removeContractDatabaseFactory');
const SVContractMongoDbRepository = require('../stateView/contract/SVContractMongoDbRepository');
const MongoDBTransaction = require('../mongoDb/MongoDBTransaction');
const SVDocumentMongoDbRepository = require('../stateView/document/mongoDbRepository/SVDocumentMongoDbRepository');
const createSVDocumentMongoDbRepositoryFactory = require('../stateView/document/mongoDbRepository/createSVDocumentMongoDbRepositoryFactory');
const convertWhereToMongoDbQuery = require('../stateView/document/mongoDbRepository/convertWhereToMongoDbQuery');
const validateQueryFactory = require('../stateView/document/query/validateQueryFactory');
const findConflictingConditions = require('../stateView/document/query/findConflictingConditions');
const applyStateTransitionFactory = require('../stateView/applyStateTransitionFactory');
const updateSVContractFactory = require('../stateView/contract/updateSVContractFactory');
const updateSVDocumentFactory = require('../stateView/document/updateSVDocumentFactory');
const DriveDataProvider = require('../dpp/DriveDataProvider');
const fetchContractFactory = require('../stateView/contract/fetchContractFactory');
const fetchDocumentsFactory = require('../stateView/document/fetchDocumentsFactory');
const BlockExecutionState = require('../updateState/BlockExecutionState');

class UpdateStateApp {
  /**
   *
   * @param {UpdateStateAppOptions} options
   */
  constructor(options) {
    this.options = options;
    this.mongoClient = null;
    this.mongoDb = null;
    this.stateViewTransaction = null;
    this.createSVDocumentRepository = null;
    this.svContractMongoDbRepository = null;
    this.blockExecutionState = new BlockExecutionState();
  }

  /**
   * Init UpdateStateApp
   * @returns {Promise<void>}
   */
  async init() {
    this.rpcClient = new RpcClient({
      protocol: 'http',
      host: this.options.getDashCoreJsonRpcHost(),
      port: this.options.getDashCoreJsonRpcPort(),
      user: this.options.getDashCoreJsonRpcUser(),
      pass: this.options.getDashCoreJsonRpcPass(),
    });

    this.mongoClient = await MongoClient.connect(
      this.options.getStorageMongoDbUrl(), {
        useNewUrlParser: true,
        useUnifiedTopology: true,
      },
    );

    this.stateViewTransaction = new MongoDBTransaction(this.mongoClient);
    this.mongoDb = this.mongoClient.db(this.options.getStorageMongoDbDatabase());

    const validateQuery = validateQueryFactory(findConflictingConditions);
    this.createSVDocumentRepository = createSVDocumentMongoDbRepositoryFactory(
      this.mongoClient,
      SVDocumentMongoDbRepository,
      convertWhereToMongoDbQuery,
      validateQuery,
    );

    this.svContractMongoDbRepository = new SVContractMongoDbRepository(
      this.mongoDb,
      new DashPlatformProtocol(),
    );
  }

  /**
   * Wraps handlers error handler for gRpc server
   *
   * @returns {Object}
   */
  createWrappedHandlers() {
    const wrapInErrorHandler = wrapInErrorHandlerFactory({
      error: errorHandler,
    });

    return {
      startTransaction: jsonToProtobufHandlerWrapper(
        jsonToProtobufFactory(
          StartTransactionRequest,
          PBJSStartTransactionRequest,
        ),
        protobufToJsonFactory(
          PBJSStartTransactionResponse,
        ),
        wrapInErrorHandler(this.createStartTransactionHandler()),
      ),
      applyStateTransition: jsonToProtobufHandlerWrapper(
        jsonToProtobufFactory(
          ApplyStateTransitionRequest,
          PBJSApplyStateTransitionRequest,
        ),
        protobufToJsonFactory(
          PBJSApplyStateTransitionResponse,
        ),
        wrapInErrorHandler(this.createApplyStateTransitionHandler()),
      ),
      commitTransaction: jsonToProtobufHandlerWrapper(
        jsonToProtobufFactory(
          CommitTransactionRequest,
          PBJSCommitTransactionRequest,
        ),
        protobufToJsonFactory(
          PBJSCommitTransactionResponse,
        ),
        wrapInErrorHandler(this.createCommitTransactionHandler()),
      ),
    };
  }

  /**
   * @private
   * @returns {startTransactionHandler}
   */
  createStartTransactionHandler() {
    return startTransactionHandlerFactory(this.stateViewTransaction);
  }

  /**
   * @private
   * @returns {applyStateTransitionHandler}
   */
  createApplyStateTransitionHandler() {
    const updateSVContract = updateSVContractFactory(this.svContractMongoDbRepository);
    const updateSVDocument = updateSVDocumentFactory(this.createSVDocumentRepository);
    const applyStateTransition = applyStateTransitionFactory(updateSVContract, updateSVDocument);
    const fetchContract = fetchContractFactory(this.svContractMongoDbRepository);
    const fetchDocuments = fetchDocumentsFactory(this.createSVDocumentRepository);

    const dataProvider = new DriveDataProvider(
      fetchDocuments,
      fetchContract,
      this.rpcClient,
      this.stateViewTransaction,
    );

    const dpp = new DashPlatformProtocol();
    dpp.setDataProvider(dataProvider);

    return applyStateTransitionHandlerFactory(
      this.stateViewTransaction,
      dpp,
      applyStateTransition,
      this.blockExecutionState,
    );
  }

  /**
   * @private
   * @returns {commitTransactionHandler}
   */
  createCommitTransactionHandler() {
    const createContractDatabase = createContractDatabaseFactory(this.createSVDocumentRepository);
    const removeContractDatabase = removeContractDatabaseFactory(this.createSVDocumentRepository);

    return commitTransactionHandlerFactory(
      this.stateViewTransaction,
      createContractDatabase,
      removeContractDatabase,
      this.blockExecutionState,
    );
  }
}

module.exports = UpdateStateApp;

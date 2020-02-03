const {
  StartTransactionRequest,
  ApplyStateTransitionRequest,
  CommitTransactionRequest,
  pbjs: {
    StartTransactionRequest: PBJSStartTransactionRequest,
    StartTransactionResponse: PBJSStartTransactionResponse,
    ApplyStateTransitionRequest: PBJSApplyStateTransitionRequest,
    ApplyStateTransitionResponse: PBJSApplyStateTransitionResponse,
    CommitTransactionRequest: PBJSCommitTransactionRequest,
    CommitTransactionResponse: PBJSCommitTransactionResponse,
  },
} = require('@dashevo/drive-grpc');

const {
  client: {
    converters: {
      jsonToProtobufFactory,
      protobufToJsonFactory,
    },
  },
  server: {
    jsonToProtobufHandlerWrapper,
    error: {
      wrapInErrorHandlerFactory,
    },
  },
} = require('@dashevo/grpc-common');

const RpcClient = require('@dashevo/dashd-rpc/promise');
const { client: JaysonClient } = require('jayson/promise');

const DashPlatformProtocol = require('@dashevo/dpp');
const { MongoClient } = require('mongodb');

const errorHandler = require('../util/errorHandler');

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
const findNotIndexedFields = require('../stateView/document/query/findNotIndexedFields');
const findNotIndexedOrderByFields = require('../stateView/document/query/findNotIndexedOrderByFields');
const getIndexedFieldsFromDocumentSchema = require('../stateView/document/query/getIndexedFieldsFromDocumentSchema');
const BlockExecutionState = require('../updateState/BlockExecutionState');
const convertToMongoDbIndices = require('../stateView/contract/convertToMongoDbIndices');

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

    this.tendermintRPCClient = JaysonClient.http({
      host: this.options.getTendermintRpcHost(),
      port: this.options.getTendermintRpcPort(),
    });

    this.mongoClient = await MongoClient.connect(
      this.options.getStateViewMongoDBUrl(), {
        useNewUrlParser: true,
        useUnifiedTopology: true,
      },
    );

    this.stateViewTransaction = new MongoDBTransaction(this.mongoClient);
    this.mongoDb = this.mongoClient.db(this.options.getStateViewMongoDBDatabase());

    const validateQuery = validateQueryFactory(
      findConflictingConditions,
      getIndexedFieldsFromDocumentSchema,
      findNotIndexedFields,
      findNotIndexedOrderByFields,
    );
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

    await this.svContractMongoDbRepository.createCollection();
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
    const fetchDocuments = fetchDocumentsFactory(
      this.createSVDocumentRepository,
      this.svContractMongoDbRepository,
    );

    const driveDpp = new DashPlatformProtocol({
      dataProvider: undefined,
    });

    const dataProvider = new DriveDataProvider(
      fetchDocuments,
      fetchContract,
      this.rpcClient,
      this.tendermintRPCClient,
      this.stateViewTransaction,
      driveDpp,
    );

    const dpp = new DashPlatformProtocol({
      dataProvider,
    });

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
    const createContractDatabase = createContractDatabaseFactory(
      this.createSVDocumentRepository,
      convertToMongoDbIndices,
    );
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

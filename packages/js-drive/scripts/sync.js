require('dotenv-expand')(require('dotenv-safe').config());

const zmq = require('zeromq');

const SyncAppOptions = require('../lib/app/SyncAppOptions');
const SyncApp = require('../lib/app/SyncApp');

const attachSyncLogger = require('../lib/sync/attachSyncLogger');
const attachSequenceValidationHandler = require('../lib/blockchain/reader/eventHandlers/attachSequenceValidationHandler');
const attachBlockErrorHandler = require('../lib/blockchain/reader/eventHandlers/attachErrorHandler');
const attachStorageHandlers = require('../lib/storage/attachStorageHandlers');
const attachSyncHandlers = require('../lib/sync/state/attachSyncHandlers');
const attachStateViewHandlers = require('../lib/stateView/attachStateViewHandlers');

const throttleFactory = require('../lib/util/throttleFactory');
const errorHandler = require('../lib/util/errorHandler');

(async function main() {
  const syncAppOptions = new SyncAppOptions(process.env);
  const syncApp = new SyncApp(syncAppOptions);
  await syncApp.init();

  const readerMediator = syncApp.createBlockchainReaderMediator();

  attachSyncLogger(
    readerMediator,
    syncApp.createLogger(),
  );

  // Attach listeners to SyncEventBus
  attachSequenceValidationHandler(
    readerMediator,
    syncApp.createStateTransitionsFromBlock(),
  );

  attachStorageHandlers(
    readerMediator,
    syncApp.createSTPacketRepository(),
  );

  attachStateViewHandlers(
    readerMediator,
    syncApp.createApplyStateTransition(),
    syncApp.createRevertSVDocumentsForStateTransition(),
    syncApp.createRevertSVContractsForStateTransition(),
    syncApp.createDropMongoDatabasesWithPrefix(),
    syncAppOptions.getMongoDbPrefix(),
  );

  attachSyncHandlers(
    readerMediator,
    syncApp.getSyncState(),
    syncApp.getSyncStateRepository(),
  );

  attachBlockErrorHandler(
    readerMediator,
    {
      skipStateTransitionWithErrors: syncAppOptions.getSyncSkipStateTransitionWithErrors(),
    },
  );

  const readBlockchainWithThrottling = throttleFactory(syncApp.createReadBlockchain());

  // Sync arriving ST packets
  const zmqSocket = zmq.createSocket('sub');
  zmqSocket.connect(syncAppOptions.getDashCoreZmqPubHashBlock());

  zmqSocket.on('message', () => {
    readBlockchainWithThrottling().catch(errorHandler);
  });

  zmqSocket.subscribe('hashblock');

  await readBlockchainWithThrottling();
}()).catch(errorHandler);

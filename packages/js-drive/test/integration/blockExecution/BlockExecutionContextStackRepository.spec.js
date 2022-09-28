const rimraf = require('rimraf');

const cbor = require('cbor');

const Drive = require('@dashevo/rs-drive');
const BlockExecutionContextStackRepository = require('../../../lib/blockExecution/BlockExecutionContextStackRepository');
const BlockExecutionContext = require('../../../lib/blockExecution/BlockExecutionContext');

const getBlockExecutionContextObjectFixture = require('../../../lib/test/fixtures/getBlockExecutionContextObjectFixture');
const BlockExecutionContextStack = require('../../../lib/blockExecution/BlockExecutionContextStack');
const GroveDBStore = require('../../../lib/storage/GroveDBStore');
const logger = require('../../../lib/util/noopLogger');

function removeConsensusLogger(blockExecutionContextStack) {
  const plainContexts = blockExecutionContextStack.getContexts()
    .map((context) => context.toObject({
      skipConsensusLogger: true,
    }));

  return plainContexts.map((rawContext) => {
    const context = new BlockExecutionContext();

    context.fromObject(rawContext);

    return context;
  });
}

describe('BlockExecutionContextStackRepository', () => {
  let rsDrive;
  let store;
  let repository;
  let blockExecutionContextObject;
  let blockExecutionContext;
  let blockExecutionContextStack;

  beforeEach(async () => {
    rsDrive = new Drive('./db/grovedb_test');
    store = new GroveDBStore(rsDrive, logger, 'blockchainStateTestStore');

    repository = new BlockExecutionContextStackRepository(store);

    blockExecutionContextObject = getBlockExecutionContextObjectFixture();

    blockExecutionContext = new BlockExecutionContext();
    blockExecutionContext.fromObject(blockExecutionContextObject);

    blockExecutionContextStack = new BlockExecutionContextStack();
    blockExecutionContextStack.setContexts([
      blockExecutionContext,
      blockExecutionContext,
    ]);
  });

  afterEach(async () => {
    await rsDrive.close();
    rimraf.sync('./db/grovedb_test');
  });

  describe('#store', () => {
    it('should store block execution context stack', async () => {
      const result = await repository.store(
        blockExecutionContextStack,
      );

      expect(result).to.equal(repository);

      const storedContextsBufferResult = await store.getAux(
        BlockExecutionContextStackRepository.EXTERNAL_STORE_KEY_NAME,
      );

      const storedContextsBuffer = storedContextsBufferResult.getValue();

      expect(storedContextsBuffer).to.be.instanceOf(Buffer);

      const storedContexts = cbor.decode(storedContextsBuffer);

      expect(storedContexts).to.deep.equals(
        blockExecutionContextStack.getContexts()
          .map((context) => context.toObject({
            skipDBTransaction: true,
            skipConsensusLogger: true,
          })),
      );
    });

    it('should store block execution context stack using transaction', async () => {
      await store.startTransaction();

      await repository.store(
        blockExecutionContextStack,
        {
          useTransaction: true,
        },
      );

      const notFoundDataResult = await store.getAux(
        BlockExecutionContextStackRepository.EXTERNAL_STORE_KEY_NAME,
        { useTransaction: false },
      );

      const notFoundData = notFoundDataResult.getValue();

      expect(notFoundData).to.be.null();

      const dataFromTransactionResult = await store.getAux(
        BlockExecutionContextStackRepository.EXTERNAL_STORE_KEY_NAME,
        { useTransaction: true },
      );

      const dataFromTransaction = dataFromTransactionResult.getValue();

      expect(dataFromTransaction).to.be.instanceOf(Buffer);

      let storedContexts = cbor.decode(dataFromTransaction);

      expect(storedContexts).to.deep.equals(
        blockExecutionContextStack.getContexts()
          .map((context) => context.toObject({
            skipDBTransaction: true,
            skipConsensusLogger: true,
          })),
      );

      await store.commitTransaction();

      const committedDataResult = await store.getAux(
        BlockExecutionContextStackRepository.EXTERNAL_STORE_KEY_NAME,
      );

      const committedData = committedDataResult.getValue();

      expect(committedData).to.be.instanceOf(Buffer);

      storedContexts = cbor.decode(committedData);

      expect(storedContexts).to.deep.equals(
        blockExecutionContextStack.getContexts()
          .map((context) => context.toObject({
            skipDBTransaction: true,
            skipConsensusLogger: true,
          })),
      );
    });
  });

  describe('#fetch', () => {
    it('should return empty block execution context stack if it is not stored', async () => {
      const storedContext = await repository.fetch();

      expect(storedContext).to.be.instanceOf(BlockExecutionContextStack);
      expect(storedContext.getSize()).to.equals(0);
    });

    it('should return stored block execution context', async () => {
      const plainContexts = blockExecutionContextStack.getContexts()
        .map((context) => context.toObject({
          skipConsensusLogger: true,
        }));

      const storedStackBuffer = cbor.encode(plainContexts);

      await store.putAux(
        BlockExecutionContextStackRepository.EXTERNAL_STORE_KEY_NAME,
        storedStackBuffer,
      );

      const storedStack = await repository.fetch();

      expect(storedStack).to.be.instanceOf(BlockExecutionContextStack);

      const blockExecutionContexts = removeConsensusLogger(blockExecutionContextStack);

      expect(storedStack.getContexts()).to.deep.equal(blockExecutionContexts);
    });

    it('should return stored block execution context using transaction', async () => {
      await store.startTransaction();

      const plainContexts = blockExecutionContextStack.getContexts()
        .map((context) => context.toObject({
          skipConsensusLogger: true,
        }));

      const storedStackBuffer = cbor.encode(plainContexts);

      await store.putAux(
        BlockExecutionContextStackRepository.EXTERNAL_STORE_KEY_NAME,
        storedStackBuffer,
        { useTransaction: true },
      );

      let storedStack = await repository.fetch({
        useTransaction: false,
      });

      expect(storedStack.getContexts()).to.deep.equal([]);

      storedStack = await repository.fetch({
        useTransaction: true,
      });

      let blockExecutionContexts = removeConsensusLogger(blockExecutionContextStack);

      expect(storedStack.getContexts()).to.deep.equal(blockExecutionContexts);

      await store.commitTransaction();

      storedStack = await repository.fetch({
        useTransaction: true,
      });

      blockExecutionContexts = removeConsensusLogger(blockExecutionContextStack);

      expect(storedStack.getContexts()).to.deep.equal(blockExecutionContexts);
    });
  });
});

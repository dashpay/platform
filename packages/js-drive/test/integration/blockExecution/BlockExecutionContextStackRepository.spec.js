const level = require('level-rocksdb');
const cbor = require('cbor');

const BlockExecutionContextStackRepository = require('../../../lib/blockExecution/BlockExecutionContextStackRepository');
const BlockExecutionContext = require('../../../lib/blockExecution/BlockExecutionContext');

const getBlockExecutionContextObjectFixture = require('../../../lib/test/fixtures/getBlockExecutionContextObjectFixture');
const BlockExecutionContextStack = require('../../../lib/blockExecution/BlockExecutionContextStack');

describe('BlockExecutionContextStackRepository', () => {
  let db;
  let repository;
  let blockExecutionContextObject;
  let blockExecutionContext;
  let blockExecutionContextStack;

  beforeEach(() => {
    db = level('./db/blockchain-state-test', { keyEncoding: 'binary', valueEncoding: 'binary' });

    repository = new BlockExecutionContextStackRepository(db);

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
    await db.clear();
    await db.close();
  });

  describe('#store', () => {
    it('should store block execution context stack', async () => {
      const result = await repository.store(
        blockExecutionContextStack,
      );

      expect(result).to.equal(repository);

      const storedContextsBuffer = await db.get(
        BlockExecutionContextStackRepository.EXTERNAL_STORE_KEY_NAME,
      );

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
          skipDBTransaction: true,
          skipConsensusLogger: true,
        }));

      const storedStackBuffer = cbor.encode(plainContexts);

      await db.put(
        BlockExecutionContextStackRepository.EXTERNAL_STORE_KEY_NAME,
        storedStackBuffer,
      );

      const storedStack = await repository.fetch();

      expect(storedStack).to.be.instanceOf(BlockExecutionContextStack);

      expect(storedStack.getContexts()).to.deep.equals(plainContexts);
    });
  });
});

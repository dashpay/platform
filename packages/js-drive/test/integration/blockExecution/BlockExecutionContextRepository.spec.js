const level = require('level-rocksdb');
const cbor = require('cbor');

const BlockExecutionContextRepository = require('../../../lib/blockExecution/BlockExecutionContextRepository');
const BlockExecutionContext = require('../../../lib/blockExecution/BlockExecutionContext');

const getBlockExecutionContextObjectFixture = require('../../../lib/test/fixtures/getBlockExecutionContextObjectFixture');

describe('BlockExecutionContextRepository', () => {
  let db;
  let repository;
  let blockExecutionContextObject;
  let blockExecutionContext;
  let key;

  beforeEach(() => {
    db = level('./db/blockchain-state-test', { keyEncoding: 'binary', valueEncoding: 'binary' });

    repository = new BlockExecutionContextRepository(db);

    blockExecutionContextObject = getBlockExecutionContextObjectFixture();

    blockExecutionContext = new BlockExecutionContext();
    blockExecutionContext.fromObject(blockExecutionContextObject);

    key = Buffer.concat([
      BlockExecutionContextRepository.EXTERNAL_STORE_KEY_NAME,
      BlockExecutionContextRepository.KEY_PREFIX_CURRENT,
    ]);
  });

  afterEach(async () => {
    await db.clear();
    await db.close();
  });

  describe('#store', () => {
    it('should store blockchain state', async () => {
      const result = await repository.store(
        BlockExecutionContextRepository.KEY_PREFIX_CURRENT,
        blockExecutionContext,
      );

      expect(result).to.equal(repository);

      const storedContextBuffer = await db.get(key);

      expect(storedContextBuffer).to.be.instanceOf(Buffer);

      const storedContext = cbor.decode(storedContextBuffer);

      expect(storedContext).to.deep.equals(
        blockExecutionContext.toObject({ skipConsensusLogger: true }),
      );
    });
  });

  describe('#fetch', () => {
    it('should return empty blockchain state if it is not stored', async () => {
      const storedContext = await repository.fetch(
        BlockExecutionContextRepository.KEY_PREFIX_CURRENT,
      );

      expect(storedContext).to.be.instanceOf(BlockExecutionContext);
      expect(storedContext.isEmpty()).to.be.true();
    });

    it('should return stored blockchain state', async () => {
      const storedContextBuffer = cbor.encode(blockExecutionContextObject);

      await db.put(key, storedContextBuffer);

      const storedContext = await repository.fetch(
        BlockExecutionContextRepository.KEY_PREFIX_CURRENT,
      );

      expect(storedContext).to.be.instanceOf(BlockExecutionContext);

      expect(storedContext.toObject({ skipConsensusLogger: true })).to.deep.equals(
        blockExecutionContext.toObject({ skipConsensusLogger: true }),
      );
    });
  });
});

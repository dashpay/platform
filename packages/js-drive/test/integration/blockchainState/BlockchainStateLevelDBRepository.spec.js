const level = require('level-rocksdb');
const cbor = require('cbor');
const Long = require('long');

const BlockchainStateLevelDBRepository = require('../../../lib/blockchainState/BlockchainStateLevelDBRepository');
const BlockchainState = require('../../../lib/blockchainState/BlockchainState');

describe('BlockchainStateLevelDBRepository', () => {
  let db;
  let repository;
  let blockchainState;
  let lastBlockHeight;
  let lastBlockAppHash;

  beforeEach(() => {
    db = level('./db/blockchain-state-test', { valueEncoding: 'binary' });

    repository = new BlockchainStateLevelDBRepository(db);

    lastBlockHeight = Long.fromInt(42);
    lastBlockAppHash = Buffer.from('something');

    blockchainState = new BlockchainState(lastBlockHeight, lastBlockAppHash);
  });

  afterEach(async () => {
    await db.clear();
    await db.close();
  });

  describe('#store', () => {
    it('should store blockchain state', async () => {
      const repositoryInstance = await repository.store(blockchainState);
      expect(repositoryInstance).to.equal(repository);

      const storedStateBuffer = await db.get(BlockchainStateLevelDBRepository.KEY_NAME);

      expect(storedStateBuffer).to.be.instanceOf(Buffer);

      const storedState = cbor.decode(storedStateBuffer);

      expect(storedState).to.have.property('lastBlockHeight', lastBlockHeight.toString());

      expect(storedState).to.have.property('lastBlockAppHash');
      expect(storedState.lastBlockAppHash).to.deep.equal(lastBlockAppHash);
    });
  });

  describe('#fetch', () => {
    it('should return empty blockchain state if it is not stored', async () => {
      const storedState = await repository.fetch();

      expect(storedState).to.be.instanceOf(BlockchainState);
      expect(storedState.getLastBlockHeight()).to.be.instanceOf(Long);
      expect(storedState.getLastBlockHeight().toInt()).to.equal(0);
      expect(storedState.getLastBlockAppHash()).to.deep.equal(Buffer.alloc(0));
    });

    it('should return stored blockchain state', async () => {
      const storedStateBuffer = cbor.encode(blockchainState.toJSON());

      await db.put(BlockchainStateLevelDBRepository.KEY_NAME, storedStateBuffer);

      const storedState = await repository.fetch();

      expect(storedState).to.be.instanceOf(BlockchainState);
      expect(storedState.getLastBlockHeight()).to.be.instanceOf(Long);
      expect(storedState.getLastBlockHeight()).to.deep.equal(lastBlockHeight);
      expect(storedState.getLastBlockAppHash()).to.deep.equal(lastBlockAppHash);
    });
  });
});

const level = require('level-rocksdb');
const cbor = require('cbor');
const Long = require('long');

const ChainInfoExternalStoreRepository = require('../../../lib/chainInfo/ChainInfoExternalStoreRepository');
const ChainInfo = require('../../../lib/chainInfo/ChainInfo');

describe('BlockchainStateLevelDBRepository', () => {
  let db;
  let repository;
  let blockchainState;
  let lastBlockHeight;

  beforeEach(() => {
    db = level('./db/blockchain-state-test', { keyEncoding: 'binary', valueEncoding: 'binary' });

    repository = new ChainInfoExternalStoreRepository(db);

    lastBlockHeight = Long.fromInt(42);

    blockchainState = new ChainInfo(lastBlockHeight);
  });

  afterEach(async () => {
    await db.clear();
    await db.close();
  });

  describe('#store', () => {
    it('should store blockchain state', async () => {
      const repositoryInstance = await repository.store(blockchainState);
      expect(repositoryInstance).to.equal(repository);

      const storedStateBuffer = await db.get(
        ChainInfoExternalStoreRepository.EXTERNAL_STORE_KEY_NAME,
      );

      expect(storedStateBuffer).to.be.instanceOf(Buffer);

      const storedState = cbor.decode(storedStateBuffer);

      expect(storedState).to.have.property('lastBlockHeight', lastBlockHeight.toString());
    });
  });

  describe('#fetch', () => {
    it('should return empty blockchain state if it is not stored', async () => {
      const storedState = await repository.fetch();

      expect(storedState).to.be.instanceOf(ChainInfo);
      expect(storedState.getLastBlockHeight()).to.be.instanceOf(Long);
      expect(storedState.getLastBlockHeight().toInt()).to.equal(0);
    });

    it('should return stored blockchain state', async () => {
      const storedStateBuffer = cbor.encode(blockchainState.toJSON());

      await db.put(ChainInfoExternalStoreRepository.EXTERNAL_STORE_KEY_NAME, storedStateBuffer);

      const storedState = await repository.fetch();

      expect(storedState).to.be.instanceOf(ChainInfo);
      expect(storedState.getLastBlockHeight()).to.be.instanceOf(Long);
      expect(storedState.getLastBlockHeight()).to.deep.equal(lastBlockHeight);
    });
  });
});

const cbor = require('cbor');

const Long = require('long');
const ChainInfoExternalStoreRepository = require('../../../lib/chainInfo/ChainInfoExternalStoreRepository');
const ChainInfo = require('../../../lib/chainInfo/ChainInfo');

describe('ChainInfoExternalStoreRepository', () => {
  let externalLevelDBMock;
  let repository;
  let chainInfo;
  let lastBlockHeight;
  let transactionMock;

  beforeEach(function beforeEach() {
    externalLevelDBMock = {
      put: this.sinon.stub(),
      get: this.sinon.stub(),
    };

    repository = new ChainInfoExternalStoreRepository(externalLevelDBMock);

    lastBlockHeight = Long.fromInt(42);

    chainInfo = new ChainInfo(lastBlockHeight);

    transactionMock = {
      db: {
        put: this.sinon.stub(),
        get: this.sinon.stub(),
      },
    };
  });

  describe('#store', () => {
    it('should store chain info', async () => {
      const repositoryInstance = await repository.store(chainInfo);
      expect(repositoryInstance).to.equal(repository);

      expect(externalLevelDBMock.put).to.be.calledOnceWithExactly(
        ChainInfoExternalStoreRepository.EXTERNAL_STORE_KEY_NAME,
        cbor.encodeCanonical(chainInfo.toJSON()),
        { asBuffer: true },
      );

      expect(transactionMock.db.put).to.be.not.called();
    });

    it('should store chain info in transaction', async () => {
      const repositoryInstance = await repository.store(chainInfo, transactionMock);
      expect(repositoryInstance).to.equal(repository);

      expect(transactionMock.db.put).to.be.calledOnceWithExactly(
        ChainInfoExternalStoreRepository.EXTERNAL_STORE_KEY_NAME,
        cbor.encodeCanonical(chainInfo.toJSON()),
        { asBuffer: true },
      );

      expect(externalLevelDBMock.put).to.be.not.called();
    });
  });

  describe('#fetch', () => {
    it('should return empty chain info if it is not stored', async () => {
      externalLevelDBMock.get.returns(null);

      const result = await repository.fetch();

      expect(result).to.be.instanceOf(ChainInfo);
      expect(result.getLastBlockHeight()).to.be.instanceOf(Long);
      expect(result.getLastBlockHeight().toInt()).to.equal(0);

      expect(externalLevelDBMock.get).to.be.calledOnceWithExactly(
        ChainInfoExternalStoreRepository.EXTERNAL_STORE_KEY_NAME,
      );
    });

    it('should return stored chain info', async () => {
      const storedStateBuffer = cbor.encode(chainInfo.toJSON());

      externalLevelDBMock.get.returns(storedStateBuffer);

      const result = await repository.fetch();

      expect(result).to.be.instanceOf(ChainInfo);
      expect(result.getLastBlockHeight()).to.be.instanceOf(Long);
      expect(result.getLastBlockHeight()).to.deep.equal(lastBlockHeight);

      expect(externalLevelDBMock.get).to.be.calledOnceWithExactly(
        ChainInfoExternalStoreRepository.EXTERNAL_STORE_KEY_NAME,
      );

      expect(transactionMock.db.get).to.be.not.called();
    });

    it('should return stored chain info in transaction', async () => {
      const storedStateBuffer = cbor.encode(chainInfo.toJSON());

      transactionMock.db.get.returns(storedStateBuffer);

      const result = await repository.fetch(transactionMock);

      expect(result).to.be.instanceOf(ChainInfo);
      expect(result.getLastBlockHeight()).to.be.instanceOf(Long);
      expect(result.getLastBlockHeight()).to.deep.equal(lastBlockHeight);

      expect(transactionMock.db.get).to.be.calledOnceWithExactly(
        ChainInfoExternalStoreRepository.EXTERNAL_STORE_KEY_NAME,
      );

      expect(externalLevelDBMock.get).to.be.not.called();
    });
  });
});

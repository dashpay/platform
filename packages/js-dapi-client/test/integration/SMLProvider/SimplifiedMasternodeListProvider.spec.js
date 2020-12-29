const SimplifiedMNList = require('@dashevo/dashcore-lib/lib/deterministicmnlist/SimplifiedMNList');

const SimplifiedMasternodeListProvider = require('../../../lib/SimplifiedMasternodeListProvider/SimplifiedMasternodeListProvider');
const DAPIAddress = require('../../../lib/dapiAddressProvider/DAPIAddress');

const getMNListDiffsFixture = require('../../../lib/test/fixtures/getMNListDiffsFixture');

const wait = require('../../../lib/test/utils/wait');

describe('SimplifiedMasternodeListProvider', () => {
  let jsonTransportMock;
  let smlProvider;
  let lastUsedAddress;
  let mnListDiffsFixture;

  beforeEach(function beforeEach() {
    lastUsedAddress = new DAPIAddress('127.0.0.1');

    jsonTransportMock = {
      request: this.sinon.stub(),
      getLastUsedAddress: this.sinon.stub().returns(lastUsedAddress),
    };

    mnListDiffsFixture = getMNListDiffsFixture();

    jsonTransportMock.request.withArgs('getBestBlockHash').onCall(0).resolves(
      mnListDiffsFixture[0].blockHash,
    );

    jsonTransportMock.request.withArgs('getBestBlockHash').onCall(1).resolves(
      mnListDiffsFixture[1].blockHash,
    );

    jsonTransportMock.request.withArgs('getMnListDiff').onCall(0).resolves(
      mnListDiffsFixture[0],
    );

    jsonTransportMock.request.withArgs('getMnListDiff').onCall(1).resolves(
      mnListDiffsFixture[1],
    );

    smlProvider = new SimplifiedMasternodeListProvider(jsonTransportMock, {
      updateInterval: 50,
      network: 'testnet',
    });
  });

  describe('#getSimplifiedMNList', () => {
    it('should update SML and return list of valid masternodes', async () => {
      expect(smlProvider.lastUpdateDate).to.equal(0);
      expect(smlProvider.baseBlockHash).to.equal(SimplifiedMasternodeListProvider.NULL_HASH);

      const sml = await smlProvider.getSimplifiedMNList();

      expect(sml).to.be.an.instanceOf(SimplifiedMNList);
      expect(sml.mnList).to.have.lengthOf(mnListDiffsFixture[0].mnList.length);

      expect(smlProvider.lastUpdateDate).to.not.equal(0);
      expect(smlProvider.baseBlockHash).to.equal(mnListDiffsFixture[0].blockHash);

      expect(jsonTransportMock.request).to.be.calledTwice();

      expect(jsonTransportMock.request.getCall(0).args).to.deep.equal([
        'getBestBlockHash',
      ]);

      expect(jsonTransportMock.request.getCall(1).args).to.deep.equal([
        'getMnListDiff',
        {
          baseBlockHash: SimplifiedMasternodeListProvider.NULL_HASH,
          blockHash: mnListDiffsFixture[0].blockHash,
        },
        {
          addresses: [lastUsedAddress],
        },
      ]);
    });

    it('should return the previous list of valid masternodes in case if update interval is not reached', async () => {
      await smlProvider.getSimplifiedMNList();

      expect(jsonTransportMock.request).to.be.calledTwice();

      // noinspection DuplicatedCode
      const sml = await smlProvider.getSimplifiedMNList();

      expect(sml).to.be.an.instanceOf(SimplifiedMNList);
      expect(sml.mnList).to.have.lengthOf(mnListDiffsFixture[0].mnList.length);

      expect(jsonTransportMock.request).to.be.calledTwice();
    });

    it('should use updated baseBlockHash for the second call', async function it() {
      this.timeout(3000);

      const firstSML = await smlProvider.getSimplifiedMNList();

      expect(firstSML).to.be.an.instanceOf(SimplifiedMNList);
      expect(firstSML.mnList).to.have.lengthOf(mnListDiffsFixture[0].mnList.length);

      expect(jsonTransportMock.request).to.be.calledTwice();

      await wait(50);

      const secondSML = await smlProvider.getSimplifiedMNList();

      expect(secondSML).to.be.an.instanceOf(SimplifiedMNList);
      expect(secondSML.mnList).to.have.lengthOf(122);

      expect(jsonTransportMock.request).to.be.callCount(4);

      expect(jsonTransportMock.request.getCall(2).args).to.deep.equal([
        'getBestBlockHash',
      ]);

      expect(jsonTransportMock.request.getCall(3).args).to.deep.equal([
        'getMnListDiff',
        {
          baseBlockHash: mnListDiffsFixture[0].blockHash,
          blockHash: mnListDiffsFixture[1].blockHash,
        },
        {
          addresses: [lastUsedAddress],
        },
      ]);
    });

    it('should reset simplifiedMNList and update masternode list from scratch', async function it() {
      this.timeout(3000);

      jsonTransportMock.request.withArgs('getBestBlockHash').onCall(1).resolves(
        mnListDiffsFixture[0].blockHash,
      );

      jsonTransportMock.request.withArgs('getMnListDiff').onCall(1).resolves(
        mnListDiffsFixture[0],
      );

      jsonTransportMock.request.withArgs('getBestBlockHash').onCall(2).resolves(
        mnListDiffsFixture[0].blockHash,
      );

      jsonTransportMock.request.withArgs('getMnListDiff').onCall(2).resolves(
        mnListDiffsFixture[0],
      );

      expect(smlProvider.lastUpdateDate).to.equal(0);
      expect(smlProvider.baseBlockHash).to.equal(SimplifiedMasternodeListProvider.NULL_HASH);

      await smlProvider.getSimplifiedMNList();
      await wait(200);

      const sml = await smlProvider.getSimplifiedMNList();

      expect(sml).to.be.an.instanceOf(SimplifiedMNList);
      expect(sml.mnList).to.have.lengthOf(mnListDiffsFixture[0].mnList.length);

      expect(jsonTransportMock.request).to.be.callCount(6);
    });
  });
});

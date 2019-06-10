const sinon = require('sinon');
const dashcoreLib = require('@dashevo/dashcore-lib')

const HeaderChainProvider = require('../../../src/headerChainSync/HeaderChainProvider');
const DAPIClient = require('../../../src/index');
const MNDiscovery = require('../../../src/MNDiscovery/index');

const RPCClient = require('../../../src/RPCClient');

const MNListFixture = require('../../fixtures/mnList');
const { testnet2: testnetHeaders } = require('../../fixtures/headers');

describe('HeaderChainProvider', function main() {
  let requestStub;
  let getRandomMasternodeStub;
  let mnListLength;
  let bestBlockHeight;

  this.timeout(10000);

  before(() => {
    mnListLength = 5;
    bestBlockHeight = 100;

    requestStub = sinon.stub(RPCClient, 'request');

    requestStub
      .withArgs({ host: sinon.match.any, port: sinon.match.any }, 'getBlockHash', sinon.match.any)
      .resolves(undefined);

    requestStub
      .withArgs({ host: sinon.match.any, port: sinon.match.any }, 'getBlockHeader', sinon.match.any)
      .resolves(new dashcoreLib.BlockHeader(new Buffer(testnetHeaders[0], 'hex')));

    requestStub
      .withArgs({ host: sinon.match.any, port: sinon.match.any }, 'getBestBlockHeight', sinon.match.any)
      .resolves(bestBlockHeight);

    getRandomMasternodeStub = sinon.stub(MNDiscovery.prototype, 'getRandomMasternode')
      .resolves(MNListFixture.getFirstDiff().mnList[0]);
  });

  describe('#sync', () => {
    it('should successfully sync headers', async () => {
      for (let i = 0; i < 100; i += 20) {
        requestStub
          .withArgs(
            { host: sinon.match.any, port: sinon.match.any },
            'getBlockHeaders',
            {
              offset: i,
              limit: 20,
              verbose: sinon.match.any,
            }
          )
          .resolves(testnetHeaders.slice(i + 1, i + 1 + 20));
      }

      const provider = new HeaderChainProvider(
        new DAPIClient({}),
        mnListLength,
        { network: 'testnet' },
      );
      const longestChain = await provider.fetch(0);

      expect(longestChain.length).to.equal(101);
    });
  });

  after(() => {
    RPCClient.request.restore();
    getRandomMasternodeStub.restore();
  });
});

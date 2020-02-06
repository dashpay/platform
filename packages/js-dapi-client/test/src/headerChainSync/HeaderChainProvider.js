const sinon = require('sinon');
const { BlockHeader } = require('@dashevo/dashcore-lib');

const HeaderChainProvider = require('../../../src/headerChainSync/HeaderChainProvider');
const DAPIClient = require('../../../src/index');

const MNListFixture = require('../../fixtures/mnList');
const { testnet2: testnetHeaders } = require('../../fixtures/headers');

describe('HeaderChainProvider', function main() {
  let mnListLength;
  let bestBlockHeight;
  let dapiClientMock;
  let dapiClient;

  this.timeout(10000);

  before(() => {
    mnListLength = 5;
    bestBlockHeight = 100;

    dapiClient = new DAPIClient({});

    const getBlockHeader = sinon
      .stub()
      .resolves(new BlockHeader(new Buffer(testnetHeaders[0], 'hex')));

    const getBlockHash = sinon
      .stub()
      .resolves(undefined);

    const getBestBlockHeight = sinon
      .stub()
      .resolves(bestBlockHeight);

    const getRandomMasternode = sinon
      .stub()
      .resolves(MNListFixture.getFirstDiff().mnList[0]);


    dapiClientMock = {
      getBlockHeader,
      getBlockHeaders: sinon.stub(),
      getBestBlockHeight,
      getBlockHash,
      getRandomMasternode,
    };
  });

  describe('#sync', () => {
    it('should successfully sync headers', async () => {
      for (let i = 0; i < 100; i += 20) {
        dapiClientMock.getBlockHeaders
          .withArgs(i, 20)
          .resolves(testnetHeaders.slice(i + 1, i + 1 + 20));
      }

      const provider = new HeaderChainProvider(
        dapiClientMock,
        mnListLength,
        { network: 'testnet' },
      );
      const longestChain = await provider.fetch(0);

      expect(longestChain.length).to.equal(101);
    });
  });
});

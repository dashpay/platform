const RpcClientMock = require('../../../../../lib/test/mock/RpcClientMock');
const getChainInfoFactory = require('../../../../../lib/sync/info/chain/getChainInfoFactory');
const ChainInfo = require('../../../../../lib/sync/info/chain/ChainInfo');

describe('getChainInfoFactory', () => {
  let rpcClient;
  let getChainInfo;

  beforeEach(function beforeEach() {
    rpcClient = new RpcClientMock(this.sinon);
    getChainInfo = getChainInfoFactory(rpcClient);
  });

  it('should return the last blockchain info', async () => {
    const lastBestBlock = rpcClient.blocks[rpcClient.blocks.length - 1];
    const chainInfo = await getChainInfo();
    expect(chainInfo).to.be.an.instanceOf(ChainInfo);
    expect(chainInfo.toJSON()).to.be.deep.equal({
      lastChainBlockHeight: lastBestBlock.height,
      lastChainBlockHash: lastBestBlock.hash,
      isBlockchainSynced: true,
    });
  });
});

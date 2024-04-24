const {
  v0: {
    GetBlockchainStatusResponse,
  },
} = require('@dashevo/dapi-grpc');

const getBlockchainStatusHandlerFactory = require('../../../../../lib/grpcServer/handlers/core/getBlockchainStatusHandlerFactory');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

describe('getBlockchainStatusHandlerFactory', () => {
  let call;
  let getBlockchainStatusHandler;
  let coreRPCClientMock;
  let now;

  let blockchainInfo;
  let networkInfo;

  beforeEach(function beforeEach() {
    blockchainInfo = {
      chain: 'test',
      blocks: 460991,
      headers: 460991,
      bestblockhash: '0000007464fd8cae97830d794bf03efbeaa4b8c3258a3def67a89cdbd060f827',
      difficulty: 0.002261509525429119,
      mediantime: 1615546573,
      verificationprogress: 0.9999993798366165,
      initialblockdownload: false,
      chainwork: '000000000000000000000000000000000000000000000000022f149b98e063dc',
      warnings: 'Warning: unknown new rules activated (versionbit 3)',
    };

    networkInfo = {
      version: 170000,
      subversion: '/Dash Core:0.17.0/',
      protocolversion: 70218,
      localservices: '0000000000000405',
      localrelay: true,
      timeoffset: 0,
      networkactive: true,
      connections: 8,
      socketevents: 'select',
      relayfee: 0.00001,
      incrementalfee: 0.00001,
      warnings: 'Warning: unknown new rules activated (versionbit 3)',
    };

    call = new GrpcCallMock(this.sinon);

    coreRPCClientMock = {
      getBlockchainInfo: this.sinon.stub().resolves(blockchainInfo),
      getNetworkInfo: this.sinon.stub().resolves(networkInfo),
    };

    now = new Date();
    this.sinon.useFakeTimers(now.getTime());

    getBlockchainStatusHandler = getBlockchainStatusHandlerFactory(coreRPCClientMock);
  });

  it('should return valid result', async () => {
    const result = await getBlockchainStatusHandler(call);

    expect(result).to.be.an.instanceOf(GetBlockchainStatusResponse);

    // Validate protobuf object values
    result.serializeBinary();

    const version = result.getVersion();
    expect(version.getProtocol()).to.equal(networkInfo.protocolversion);
    expect(version.getSoftware()).to.equal(networkInfo.version);
    expect(version.getAgent()).to.equal(networkInfo.subversion);

    const time = result.getTime();
    expect(time.getNow()).to.be.an('number');
    expect(time.getNow()).to.equal(Math.floor(now.getTime() / 1000));
    expect(time.getOffset()).to.be.equal(networkInfo.timeoffset);
    expect(time.getMedian()).to.be.equal(blockchainInfo.mediantime);

    const chain = result.getChain();
    expect(chain.getName()).to.be.equal(blockchainInfo.chain);
    expect(chain.getBlocksCount()).to.be.equal(blockchainInfo.blocks);
    expect(chain.getHeadersCount()).to.be.equal(blockchainInfo.headers);
    expect(chain.getBestBlockHash()).to.be.an.instanceOf(Buffer);
    expect(chain.getBestBlockHash().toString('hex')).to.be.equal(blockchainInfo.bestblockhash);
    expect(chain.getDifficulty()).to.be.equal(blockchainInfo.difficulty);
    expect(chain.getChainWork()).to.be.an.instanceOf(Buffer);
    expect(chain.getChainWork().toString('hex')).to.be.equal(blockchainInfo.chainwork);
    expect(chain.getIsSynced()).to.be.equal(blockchainInfo.verificationprogress === 1);
    expect(chain.getSyncProgress()).to.be.equal(blockchainInfo.verificationprogress);

    const network = result.getNetwork();
    expect(network.getPeersCount()).to.be.equal(networkInfo.connections);

    const fee = network.getFee();
    expect(fee.getRelay()).to.be.equal(networkInfo.relayfee);
    expect(fee.getIncremental()).to.be.equal(networkInfo.incrementalfee);

    expect(result.getStatus()).to.be.equal(GetBlockchainStatusResponse.Status.SYNCING);
    expect(coreRPCClientMock.getBlockchainInfo).to.be.calledOnce();
    expect(coreRPCClientMock.getNetworkInfo).to.be.calledOnce();
  });
});

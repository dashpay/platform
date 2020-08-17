const {
  v0: {
    GetStatusResponse,
  },
} = require('@dashevo/dapi-grpc');

const getStatusHandlerFactory = require('../../../../../lib/grpcServer/handlers/core/getStatusHandlerFactory');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

describe('getStatusHandlerFactory', () => {
  let call;
  let getStatusHandler;
  let insightAPIMock;
  let info;

  beforeEach(function beforeEach() {
    info = {
      version: 140001,
      insightversion: '2.3.3',
      protocolversion: 70215,
      blocks: 1185683,
      timeoffset: 0,
      connections: 8,
      proxy: '',
      difficulty: 184564887.5403167,
      testnet: false,
      relayfee: 0.00001,
      errors: '',
      network: 'livenet',
    };

    call = new GrpcCallMock(this.sinon);

    insightAPIMock = {
      getStatus: this.sinon.stub().resolves({ info }),
    };

    getStatusHandler = getStatusHandlerFactory(insightAPIMock);
  });

  it('should return valid result', async () => {
    const result = await getStatusHandler(call);

    expect(result).to.be.an.instanceOf(GetStatusResponse);

    expect(result.getCoreVersion()).to.equal(info.version);
    expect(result.getProtocolVersion()).to.equal(info.protocolversion);
    expect(result.getBlocks()).to.equal(info.blocks);
    expect(result.getTimeOffset()).to.equal(info.timeoffset);
    expect(result.getConnections()).to.equal(info.connections);
    expect(result.getProxy()).to.equal(info.proxy);
    expect(result.getDifficulty()).to.equal(info.difficulty);
    expect(result.getTestnet()).to.equal(info.testnet);
    expect(result.getRelayFee()).to.equal(info.relayfee);
    expect(result.getErrors()).to.equal(info.errors);
    expect(result.getNetwork()).to.equal(info.network);

    expect(insightAPIMock.getStatus).to.be.calledOnceWith('getInfo');
  });
});

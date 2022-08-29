const getConsensusParamsFactory = require('../../../../lib/externalApis/tenderdash/getConsensusParamsFactory');
const RPCError = require('../../../../lib/rpcServer/RPCError');

describe('getConsensusParamsFactory', () => {
  let getConsensusParams;
  let rpcClientMock;
  let response;

  beforeEach(function beforeEach() {
    response = {
      id: '',
      jsonrpc: '2.0',
      error: '',
      result: {
        consensus_params: {
          block: {
            max_bytes: '22020096',
            max_gas: '1000',
            time_iota_ms: '1000',
          },
          evidence: {
            max_age_num_blocks: '100000',
            max_age_duration: '200000',
            max_bytes: '22020096',
          },
          validator: {
            pub_key_types: [
              'ed25519',
            ],
          },
        },
      },
    };

    rpcClientMock = {
      request: this.sinon.stub().resolves(response),
    };

    getConsensusParams = getConsensusParamsFactory(rpcClientMock);
  });

  it('should return valid result', async () => {
    const result = await getConsensusParams(42);

    expect(result).to.deep.equal({
      block: response.result.consensus_params.block,
      evidence: response.result.consensus_params.evidence,
    });

    expect(rpcClientMock.request).to.be.calledOnceWith('consensus_params', { height: '42' });
  });

  it('should throw RPCError', async () => {
    rpcClientMock.request.resolves({
      id: '',
      jsonrpc: '2.0',
      result: {},
      error: {
        code: -32601,
        message: 'internal error',
        data: 'additional data',
      },
    });

    try {
      await getConsensusParams();

      expect.fail('should throw RPCError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(RPCError);
      expect(e.code).to.equal(-32601);
      expect(e.message).to.equal('internal error');
      expect(e.data).to.equal('additional data');
    }
  });
});

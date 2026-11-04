import createRpcClient from '../../../../src/core/createRpcClient.js';
import registerMasternode from '../../../../src/core/wallet/registerMasternode.js';

describe('registerMasternode', () => {
  const collateralHash = 'a'.repeat(64);

  let rpcClient;
  let coreService;
  let config;
  let fetchStub;

  beforeEach(function beforeEach() {
    rpcClient = createRpcClient({ user: 'dashmate', pass: 'secret', port: 19998 });
    rpcClient.masternode = this.sinon.stub()
      .withArgs('outputs', { wallet: 'main' })
      .resolves({ result: [`${'b'.repeat(64)}-0`, `${collateralHash}-1`] });

    coreService = { getRpcClient: () => rpcClient };

    const options = {
      externalIp: '127.0.0.1',
      'core.p2p.port': 20001,
      'platform.drive.tenderdash.node.id': 'c'.repeat(40),
      'platform.drive.tenderdash.p2p.port': 26656,
      'platform.gateway.listeners.dapiAndDrive.port': 1443,
    };
    config = { get: (key) => options[key] };

    fetchStub = this.sinon.stub(globalThis, 'fetch').resolves(new Response(
      JSON.stringify({ result: 'd'.repeat(64), error: null, id: 1 }),
    ));
  });

  it('should send collateralIndex as a number, as Core v24 requires', async () => {
    // Core 24.0.0-rc.6 rejects a string collateralIndex with:
    // "Position 2 (collateralIndex)": "JSON value of type string is not of expected type number"
    const proRegTxId = await registerMasternode(
      coreService,
      collateralHash,
      'yOwnerAddress',
      'operatorPublicKey',
      'yFundSourceAddress',
      '10.00',
      config,
      true,
    );

    expect(proRegTxId).to.equal('d'.repeat(64));
    expect(fetchStub).to.have.been.calledOnce();

    const [url, { body }] = fetchStub.firstCall.args;
    expect(url).to.equal('http://127.0.0.1:19998/wallet/main');

    const { method, params } = JSON.parse(body);
    expect(method).to.equal('protx');
    expect(params).to.deep.equal([
      'register_evo',
      collateralHash,
      1,
      '127.0.0.1:20001',
      'yOwnerAddress',
      'operatorPublicKey',
      'yOwnerAddress',
      '10.00',
      'yFundSourceAddress',
      'c'.repeat(40),
      '26656',
      '1443',
    ]);
  });
});

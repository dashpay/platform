import { EventEmitter } from 'events';
import fs from 'fs';
import http from 'http';
import path from 'path';
import { PACKAGE_ROOT_DIR } from '../../../src/constants.js';
import createRpcClient from '../../../src/core/createRpcClient.js';

describe('createRpcClient', () => {
  it('should call getdeploymentinfo, where Core v24 reports platform activation', async function it() {
    // Recorded from Dash Core 24.0.0-rc.3 on regtest with testactivationheight=mn_rr@1000
    const deploymentInfo = JSON.parse(fs.readFileSync(
      path.join(PACKAGE_ROOT_DIR, 'test/fixtures/coreRpc/24.0.0-rc.3/getdeploymentinfo-1112.json'),
      'utf8',
    ));

    let requestBody = '';

    this.sinon.stub(http, 'request').callsFake((options, callback) => {
      const request = new EventEmitter();
      request.setHeader = () => {};
      request.write = (data) => {
        requestBody += data;
      };
      request.end = () => {
        const response = new EventEmitter();
        callback(response);
        response.statusCode = 200;
        response.emit('data', JSON.stringify({ result: deploymentInfo, error: null, id: 1 }));
        response.emit('end');
      };

      return request;
    });

    const { result } = await createRpcClient().getDeploymentInfo();

    const { method, params } = JSON.parse(requestBody);
    expect(method).to.equal('getdeploymentinfo');
    expect(params).to.deep.equal([]);
    expect(result.deployments.mn_rr).to.deep.equal({ type: 'buried', active: true, height: 1000 });
  });

  describe('callWithParams', () => {
    it('should send params without converting their types', async function it() {
      const fetchStub = this.sinon.stub(globalThis, 'fetch').resolves(new Response(
        JSON.stringify({ result: 'ok', error: null, id: 1 }),
      ));

      const rpcClient = createRpcClient({ user: 'dashmate', pass: 'secret', port: 19998 });
      rpcClient.setTimeout(1000);

      const response = await rpcClient.callWithParams('protx', ['register', 'hash', 1], { wallet: 'main' });

      expect(response.result).to.equal('ok');

      const [url, { headers, body, signal }] = fetchStub.firstCall.args;
      expect(url).to.equal('http://127.0.0.1:19998/wallet/main');
      expect(headers.Authorization).to.equal(`Basic ${Buffer.from('dashmate:secret').toString('base64')}`);
      expect(JSON.parse(body)).to.deep.equal({ id: 1, method: 'protx', params: ['register', 'hash', 1] });
      expect(signal).to.be.an.instanceOf(AbortSignal);
    });

    it('should throw the Core RPC error with its code', async function it() {
      // Dash Core 24.0.0-rc.6 replies to a string collateralIndex with HTTP 500 and this error
      const message = 'Wrong type passed:\n{\n    "Position 2 (collateralIndex)": "JSON value of type string is not of expected type number"\n}';
      this.sinon.stub(globalThis, 'fetch').resolves(new Response(
        JSON.stringify({ result: null, error: { code: -3, message }, id: 1 }),
        { status: 500 },
      ));

      const error = await createRpcClient().callWithParams('protx', ['register', 'hash', '1'])
        .catch((e) => e);

      expect(error).to.be.an.instanceOf(Error);
      expect(error.message).to.equal(message);
      expect(error.code).to.equal(-3);
    });

    it('should throw the HTTP status when Core does not reply with JSON', async function it() {
      this.sinon.stub(globalThis, 'fetch').resolves(new Response('', { status: 401 }));

      await expect(createRpcClient().callWithParams('getblockcount', []))
        .to.be.rejectedWith('Dash JSON-RPC: HTTP 401: ');
    });
  });
});

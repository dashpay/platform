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
});

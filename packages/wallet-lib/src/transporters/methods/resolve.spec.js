const { expect } = require('chai');
const transporters = require('../index');

describe('transporters', () => {
  it('should resolve dapi as default transporter', () => {
    const defaultTransporter = transporters.resolve();
    expect(defaultTransporter).to.be.instanceOf(transporters.DAPIClient);
  });
  it('should resolve transporters from string', () => {
    const opts = 'dapi';
    const dapiTransporter = transporters.resolve(opts);
    expect(dapiTransporter).to.be.instanceOf(transporters.DAPIClient);

    const rpcTransporter = transporters.resolve('RPCCLient');
    expect(rpcTransporter).to.be.instanceOf(transporters.RPCClient);
  });
  it('should resolve transporters from object', () => {
    const optsRPC = {
      type: 'RPC',
      ip: '0.0.0.0',
    };
    const optsDAPI = {
      type: 'DAPIClient',
      seeds: [{ service: '18.236.131.253:3000' }],
    };
    const dapiTransporter = transporters.resolve(optsDAPI);
    expect(dapiTransporter).to.be.instanceOf(transporters.DAPIClient);

    const rpcTransporter = transporters.resolve(optsRPC);
    expect(rpcTransporter).to.be.instanceOf(transporters.RPCClient);


    const dapiTransporter2 = transporters.resolve({ type: 'dapi', seeds: [{ service: '18.236.131.253:3000' }] });
    expect(dapiTransporter2).to.be.instanceOf(transporters.DAPIClient);
    expect(dapiTransporter2.type).to.be.equal('DAPIClient');
  });
});

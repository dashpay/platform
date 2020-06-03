const { expect } = require('chai');
const transporters = require('../index');

describe('transporters', function suite() {
  this.timeout(10000);
  it('should resolve dapi as default transporter', () => {
    const defaultTransporter = transporters.resolve();
    expect(defaultTransporter).to.be.instanceOf(transporters.DAPIClientWrapper);

    const opts = {
      seeds: [{ service: '18.236.131.253:3000' }],
    }
    const defaultTransporterWithOpts = transporters.resolve(opts);
    expect(defaultTransporterWithOpts).to.be.instanceOf(transporters.DAPIClientWrapper);
    expect(defaultTransporterWithOpts.client.MNDiscovery.seeds).to.be.deep.equal(opts.seeds);
  });
  it('should resolve transporters from string', () => {
    const opts = 'dapi';
    const dapiTransporter = transporters.resolve(opts);
    expect(dapiTransporter).to.be.instanceOf(transporters.DAPIClientWrapper);

    const rpcTransporter = transporters.resolve('RPCClient');
    expect(rpcTransporter).to.be.instanceOf(transporters.RPCClient);
    expect(rpcTransporter.ip).to.equal('127.0.0.1');
  });
  it('should resolve transporters from object', () => {
    const optsRPC = {
      type: 'RPC',
      ip: '0.0.0.0',
    };
    const rpcTransporter = transporters.resolve(optsRPC);
    expect(rpcTransporter).to.be.instanceOf(transporters.RPCClient);
    expect(rpcTransporter.ip).to.equal(optsRPC.ip);

    const optsDAPI = {
      type: 'DAPIClient' ,
    };
    const dapiTransporter = transporters.resolve(optsDAPI);
    expect(dapiTransporter).to.be.instanceOf(transporters.DAPIClientWrapper);

    const optsDAPIWithSeeds = {
      type: 'DAPIClientWrapper',
      seeds: [{ service: '18.236.131.253:3000' }],
    };
    const dapiTransporterWithSeeds = transporters.resolve(optsDAPIWithSeeds);
    expect(dapiTransporterWithSeeds).to.be.instanceOf(transporters.DAPIClientWrapper);
    expect(dapiTransporterWithSeeds.client.MNDiscovery.seeds).to.be.deep.equal(optsDAPIWithSeeds.seeds);

    const optsDAPI2 = { type: 'dapi', seeds: [{ service: '18.236.131.254' }] };
    const dapiTransporter2 = transporters.resolve(optsDAPI2);
    expect(dapiTransporter2).to.be.instanceOf(transporters.DAPIClientWrapper);
    expect(dapiTransporter2.type).to.be.equal('DAPIClientWrapper');
    expect(dapiTransporter2.client.MNDiscovery.seeds).to.be.deep.equal(optsDAPI2.seeds);
  });
  it('should extend passed options', () => {
    const options = {
      type: 'DAPIClientWrapper',
      seeds: [{ service: '123.4.5.6' }],
    };

    const transporter = transporters.resolve(options);
    expect(transporter.type).to.be.equal('DAPIClientWrapper');
    expect(transporter.client.MNDiscovery.seeds).to.be.deep.equal(options.seeds);
  });
  it('should resolves the transporter passed as a props', () => {
    const opts = {
      seeds: [{ service: '123.4.5.6' }],
      timeout: 1000,
      retries: 5,
      network: 'testnet',
    };
    const client = new transporters.DAPIClientWrapper(opts);
    const transporter = transporters.resolve(client);
    expect(transporter).to.deep.equal(client);
    expect(transporter.client.MNDiscovery.seeds).to.be.deep.equal(opts.seeds);
  });
});

const { expect } = require('chai');
const DAPIClient = require('@dashevo/dapi-client');
const Transporter = require('../../src/transports/Transporter');
const InsightClient = require('../../src/transports/Insight/insightClient');

const pluginRequiredKeys = ['getAddressSummary', 'getTransactionById', 'getUTXO', 'subscribeToAddresses', 'closeSocket', 'sendRawTransaction'];

describe('Transporter', () => {
  it('should create a new transporter', () => {
    const transporterDAPI = new Transporter('dapi');
    const transporterInsight = new Transporter('insight');

    expect(transporterDAPI.isValid).to.equal(true);
    expect(transporterInsight.isValid).to.equal(true);

    const dapiClient = new DAPIClient();
    const insightClient = new InsightClient();

    const transporterDAPI2 = new Transporter(dapiClient);
    const transporterInsight2 = new Transporter(insightClient);
    expect(transporterDAPI2.isValid).to.equal(true);
    expect(transporterInsight2.isValid).to.equal(false);

    const fakeTransportPlugin = {};
    [...pluginRequiredKeys]
      .forEach((key) => {
        fakeTransportPlugin[key] = function () {
          return new Error('DummyFunction');
        };
      });
    const transporterFake = new Transporter(fakeTransportPlugin);
    expect(transporterFake.isValid).to.equal(true);

    const fakeTransportPlugin2 = {};
    pluginRequiredKeys.forEach((key) => {
      fakeTransportPlugin2[key] = function () {
        return new Error('DummyFunction');
      };
    });
    const transporterFake2 = new Transporter(fakeTransportPlugin2);
    expect(transporterFake2.isValid).to.equal(true);

    transporterDAPI.disconnect();
    transporterInsight.disconnect();
    transporterDAPI2.disconnect();
    transporterInsight2.disconnect();
    transporterFake.disconnect();
    transporterFake2.disconnect();
  });
  it('should handle invalid transporter', () => {
    const empty = new Transporter('tirelipinpon');
    expect(empty.isValid).to.equal(false);
    const invalid = new Transporter('invalidName');
    expect(invalid.isValid).to.equal(false);

    const fakeTransportPlugin = {};
    [...pluginRequiredKeys.slice(0, pluginRequiredKeys.length - 1)]
      .forEach((key) => {
        fakeTransportPlugin[key] = function () {
          return new Error('DummyFunction');
        };
      });
    const transporterFake = new Transporter(fakeTransportPlugin);
    expect(transporterFake.isValid).to.equal(false);

    empty.disconnect();
    invalid.disconnect();
    transporterFake.disconnect();
  });
  it('should handle the change of a network', () => {
    // todo : not bhandled in DAPIClient, need in transporter
    // const dapiClient = new DAPIClient();
    // const transport = new Transporter(dapiClient);
    // console.log(transport.getNetwork())
    // expect(transport.getNetwork().toString()).to.equal('testnet');
    // transport.updateNetwork('livenet');
    // expect(transport.getNetwork().toString()).to.equal('livenet');
    // transport.disconnect();
    //
    // const fakeTransportPlugin = {};
    // [...pluginRequiredKeys]
    //   .forEach((key) => {
    //     fakeTransportPlugin[key] = function () {
    //       return new Error('DummyFunction');
    //     };
    //   });
    //
    // const transporterFake = new Transporter(fakeTransportPlugin);
    // expect(() => transporterFake.updateNetwork('livenet')).to.throw('Transport does not handle network changes');
    // transport.disconnect();
  });
  it('should handle sendRawTransaction', async () => {
    const insightClient = new InsightClient();
    const transport = new Transporter(insightClient);

    return transport.sendRawTransaction(1234)
      .then(() => Promise.reject(new Error('Expected method to reject.')))
      .catch((err) => {
        expect(err.toString()).to.be.equal(new Error('Received an invalid rawtx').toString());
        transport.disconnect();
      });
  });
  it('should handle getUTXO', async () => {
    const insightClient = new InsightClient();
    const transport = new Transporter(insightClient);

    return transport.getUTXO(123)
      .then(() => Promise.reject(new Error('Expected method to reject.')))
      .catch((err) => {
        expect(err.toString()).to.be.equal(new Error('Received an invalid address to fetch').toString());
        transport.disconnect();
      });
  });
  it('should handle subscribeToEvent', async () => {
    const insightClient = new InsightClient();
    const transport = new Transporter(insightClient);

    return transport.subscribeToEvent(null)
      .then((res) => {
        expect(res).to.be.equal(false);
        transport.disconnect();
      });
  });
});

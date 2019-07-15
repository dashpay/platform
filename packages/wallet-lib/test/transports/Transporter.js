const { expect } = require('chai');
const DAPIClient = require('@dashevo/dapi-client');
const Transporter = require('../../src/transports/Transporter');

const pluginRequiredKeys = ['getAddressSummary', 'getTransactionById', 'getUTXO', 'subscribeToAddresses', 'closeSocket', 'sendRawTransaction'];

describe('Transporter', () => {
  it('should create a new transporter', () => {
    const transporterDAPI = new Transporter('dapi');

    expect(transporterDAPI.isValid).to.equal(true);

    const dapiClient = new DAPIClient();

    const transporterDAPI2 = new Transporter(dapiClient);
    expect(transporterDAPI2.isValid).to.equal(true);

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
    transporterDAPI2.disconnect();
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
});

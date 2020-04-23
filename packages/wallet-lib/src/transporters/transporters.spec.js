const { expect } = require('chai');
const DAPIClient = require('@dashevo/dapi-client');
const transporters = require('./index');

const pluginRequiredKeys = [
  'getAddressSummary',
  'getTransaction',
  'getUTXO',
  'subscribeToAddresses',
];

describe('Transporter', () => {
  it('should create a new transporter', () => {
    const transporterDAPI = transporters.resolve('dapi');
    expect(transporterDAPI.isValid).to.equal(true);
    const dapiClient = new DAPIClient();

    const transporterDAPI2 = transporters.resolve(dapiClient);
    expect(transporterDAPI2.isValid).to.equal(true);

    const fakeTransportPlugin = {};
    [...pluginRequiredKeys]
      .forEach((key) => {
        fakeTransportPlugin[key] = function () {
          return new Error('DummyFunction');
        };
      });
    const transporterFake = transporters.resolve(fakeTransportPlugin);
    expect(transporterFake.isValid).to.equal(false);

    const fakeTransportPlugin2 = {};
    pluginRequiredKeys.forEach((key) => {
      fakeTransportPlugin2[key] = function () {
        return new Error('DummyFunction');
      };
    });
    const transporterFake2 = transporters.resolve(fakeTransportPlugin2);
    expect(transporterFake2.isValid).to.equal(false);

    transporterDAPI.disconnect();
    // transporterDAPI2.disconnect();
    // transporterFake.disconnect();
    // transporterFake2.disconnect();
  });
  it('should handle invalid transporter', () => {
    const empty = transporters.resolve('tirelipinpon');
    expect(empty.isValid).to.equal(false);
    const invalid = transporters.resolve('invalidName');
    expect(invalid.isValid).to.equal(false);

    class FakeTransporter {
      constructor() {
        this.type = 'FakeTransporter';
        [...pluginRequiredKeys.slice(0, pluginRequiredKeys.length - 1)]
          .forEach((key) => {
            this[key] = function () {
              return new Error('DummyFunction');
            };
          });
      }
    }
    const transporterFake = transporters.resolve(FakeTransporter);
    expect(transporterFake.isValid).to.equal(false);

    empty.disconnect();
    invalid.disconnect();
    // transporterFake.disconnect();
  });
});

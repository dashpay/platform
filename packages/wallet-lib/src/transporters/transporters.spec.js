const { expect } = require('chai');
const DAPIClient = require('@dashevo/dapi-client');
const transporters = require('./index');

const FakeInvalidTransporter = require('../../fixtures/transporters/FakeInvalidTransporter');
const FakeValidTransporter = require('../../fixtures/transporters/FakeValidTransporter');
describe('Transporter', function suite() {
  this.timeout(10000);
  it('should create a new transporter', () => {
    const transporterDAPI = transporters.resolve('dapi');
    expect(transporterDAPI.isValid).to.equal(true);
    const dapiClient = new DAPIClient();

    const transporterDAPI2 = transporters.resolve(dapiClient);
    expect(transporterDAPI2.isValid).to.equal(true);

    const transporterInvalidFake = transporters.resolve(FakeInvalidTransporter);
    expect(transporterInvalidFake.isValid).to.equal(false);

    const transporterValidFake = transporters.resolve(FakeValidTransporter);
    expect(transporterValidFake.isValid).to.equal(true);
  });
  it('should handle invalid transporter', () => {
    const expectedExpection = 'Not supported : Transport dummmyName';
    expect(()=>transporters.resolve('dummmyName')).to.throw(expectedExpection);
  });
});

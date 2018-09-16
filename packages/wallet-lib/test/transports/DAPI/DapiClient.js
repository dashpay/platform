const { expect } = require('chai');
const DAPIClient = require('../../../src/transports/DAPI/DapiClient');

describe('Transports - DAPIClient', () => {
  it('should create a DAPIClient object', () => {
    const result = new DAPIClient();
    const expectedResult = 'DAPIClient';
    expect(result.constructor.name).to.equal(expectedResult);
    expect(result.type).to.equal(expectedResult);
  });
  it('should handle getAddressSummary', () => {
    const client = new DAPIClient();
    return client
      .getAddressSummary()
      .then((res) => { expect(res).to.be.equal(false); })
      .catch(() => Promise.reject(new Error('Expected method to return false.')));
  });
  it('should handle getStatus', () => {
    const client = new DAPIClient();
    return client
      .getStatus()
      .then((res) => { expect(res).to.be.equal(false); })
      .catch(() => Promise.reject(new Error('Expected method to return false.')));
  });
  it('should handle getTransaction', () => {
    const client = new DAPIClient();
    return client
      .getTransaction()
      .then((res) => { expect(res).to.be.equal(false); })
      .catch(() => Promise.reject(new Error('Expected method to return false.')));
  });
  it('should handle getUTXO', () => {
    const client = new DAPIClient();
    return client
      .getUTXO()
      .then((res) => { expect(res).to.be.equal(false); })
      .catch(() => Promise.reject(new Error('Expected method to return false.')));
  });
  it('should handle subscribeToAddresses', () => {
    const client = new DAPIClient();

    return client
      .subscribeToAddresses()
      .then((res) => { expect(res).to.be.equal(false); })
      .catch(() => Promise.reject(new Error('Expected method to return false.')));
  });
  it('should handle subscribeToEvent', () => {
    const client = new DAPIClient();

    return client
      .subscribeToEvent()
      .then((res) => { expect(res).to.be.equal(false); })
      .catch(() => Promise.reject(new Error('Expected method to return false.')));
  });
  it('should handle unsubscribeFromEvent', () => {
    const client = new DAPIClient();

    return client
      .unsubscribeFromEvent()
      .then((res) => { expect(res).to.be.equal(false); })
      .catch(() => Promise.reject(new Error('Expected method to return false.')));
  });
  it('should handle sendRawTransaction', () => {
    const client = new DAPIClient();

    return client
      .sendRawTransaction()
      .then((res) => { expect(res).to.be.equal(false); })
      .catch(() => Promise.reject(new Error('Expected method to return false.')));
  });
  it('should handle updateNetwork', () => {
    const client = new DAPIClient();

    return client
      .updateNetwork()
      .then((res) => { expect(res).to.be.equal(false); })
      .catch(() => Promise.reject(new Error('Expected method to return false.')));
  });
  it('should handle closeSocket', () => {
    const client = new DAPIClient();

    return client
      .closeSocket()
      .then((res) => { expect(res).to.be.equal(false); })
      .catch(() => Promise.reject(new Error('Expected method to return false.')));
  });
});

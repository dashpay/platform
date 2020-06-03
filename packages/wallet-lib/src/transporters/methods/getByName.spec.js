const { expect } = require('chai');
const transporters = require('../index');

describe('transporters',function suite() {
  this.timeout(10000);
  it('should warn on unfound Transporter class', () => {
    const expectedException1 = 'Not supported : Transport StarlinkClient';
    expect(() => transporters.getByName('StarlinkClient')).to.throw(expectedException1);
  });
  it('should get Transporter class by name', () => {
    expect(transporters.getByName('dapi')).to.equal(transporters.DAPIClientWrapper);
    expect(transporters.getByName('DAPI')).to.equal(transporters.DAPIClientWrapper);
    expect(transporters.getByName('DAPIClient')).to.equal(transporters.DAPIClientWrapper);
    expect(transporters.getByName('DAPIClientWrapper')).to.equal(transporters.DAPIClientWrapper);
  });
});

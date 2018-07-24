const expect = require('chai').expect;

const DAPIClient = require('../../dist/bundle.js');

describe('dist/bundle.js', () => {
  it('Common.js exports should export DAPIClient class', () => {
    const dapiClient = new DAPIClient();
    expect(dapiClient).to.have.property('makeRequestToRandomDAPINode');
  });
});
const SDK = require('../../src/index');
const { expect } = require('chai');

describe('SDK', () => {
  it('Should export Api', () => {
    expect(SDK).to.have.property('Api');
  });
  it('Should export Core', () => {
    expect(SDK).to.have.property('Core');
  });
  it('Should export Bitcore', () => {
    expect(SDK).to.have.property('Bitcore');
  });
});

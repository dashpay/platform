const expect = require('chai').expect;

const bundle = require('../../dist/bundle.js');

const expectedExports = ['Api', 'Core', 'Bitcore'];

describe('dist/bundle.js', () => {
  it('Common.js exports should be the same as src/index', () => {
    expectedExports.forEach((member) => {
      expect(bundle).to.have.property(member);
    });
  });
});
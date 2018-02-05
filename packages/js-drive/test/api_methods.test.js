const { expect } = require('chai');

const {
  getBlockchainUser,
} = require('../lib/api_methods');

describe('API.getBlockchainUser', () => {
  it('should return a blockchain user', () => {
    getBlockchainUser({ name: 'andy' }, (err, name) => {
      expect(name).to.be.a('string');
      expect(name).to.be.eql('Got BU: andy');
    });
  });
});

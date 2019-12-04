const {expect} = require('chai');
const dpp = require('@dashevo/dpp');
const schema = require('../schema/dashpay.schema');

describe('Schema', () => {
  it('should be a valid contract', function () {
    const validationResult = new dpp().dataContract.validate(schema);
    expect(validationResult.errors).to.deep.equal([]);
  });
});

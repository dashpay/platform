const { expect } = require('chai');
const feeCalculation = require('./feeCalculation');

describe('Utils - feeCalculation', () => {
  it('should get feeRate for an instantSend transaction', () => {
    const result = feeCalculation('instantSend');
    const expectedResult = {
      type: 'perInputs',
      value: 10000,
    };
    expect(result).to.deep.equal(expectedResult);
  });
  it('should get feeRate for an classic transaction', () => {
    const result1 = feeCalculation('standard');
    const result2 = feeCalculation();

    const expectedResult = {
      type: 'perBytes',
      value: 1000,
    };
    expect(result1).to.deep.equal(expectedResult);
    expect(result2).to.deep.equal(expectedResult);
  });
});

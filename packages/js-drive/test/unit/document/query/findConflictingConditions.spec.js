const findConflictingConditions = require('../../../../lib/document/query/findConflictingConditions');

const allowedPairsTestCases = [
  ['<', '>'],
  ['<', '>='],
  ['>', '<'],
  ['>', '<='],
];
const restrictedPairsTestCases = [
  ['==', '<'],
  ['<', '<'],
  ['>', '>'],
  ['=>', '<='],
  ['==', '=='],
];

describe('findConflictingConditions', () => {
  it('should return an empty array if field used with only one operator', () => {
    const result = findConflictingConditions([['field', '==', 'value'], ['field2', '<', 'value2']]);

    expect(result).to.be.an('array');
    expect(result).to.have.lengthOf(0);
  });

  allowedPairsTestCases.forEach(([operator1, operator2]) => {
    it(`should return an empty array if field used with "${operator1}" and "${operator2}" operators`, () => {
      const result = findConflictingConditions([['a', operator1, 1], ['a', operator2, 1]]);

      expect(result).to.be.an('array');
      expect(result).to.have.lengthOf(0);
    });
  });

  restrictedPairsTestCases.forEach(([operator1, operator2]) => {
    it(`should return an array with a field if field used with "${operator1}" and "${operator2}" operators`, () => {
      const result = findConflictingConditions([['a', operator1, 1], ['a', operator2, 1]]);

      expect(result).to.be.an('array');
      expect(result).to.have.lengthOf(1);
      expect(result[0]).to.be.deep.equal(['a', [operator1, operator2]]);
    });
  });

  it('should return array with field if it used with more than two operators', () => {
    const result = findConflictingConditions([['a', '>', 1], ['a', '<', 1], ['a', '=>', 1]]);

    expect(result).to.be.an('array');
    expect(result).to.have.lengthOf(1);
    expect(result[0]).to.be.deep.equal(['a', ['>', '<', '=>']]);
  });
});

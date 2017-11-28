const { expect } = require('chai');

const listUtils = require('../../lib/utils/listUtils');

// Disable no-undef rule for mocha
/* eslint-disable no-undef */
describe('listUtils', () => {
  describe('.getDiff', () => {
    it('should return correct diffs', () => {
      const oldList = [{ vin: '1' }, { vin: '2' }];
      const newList = [{ vin: '1' }, { vin: '3' }];
      const listDiff = listUtils.getDiff(oldList, newList);
      expect(listDiff).to.be.an('object');
      expect(listDiff).to.have.property('additions');
      expect(listDiff).to.have.property('deletions');
      expect(listDiff.additions).to.be.an('array');
      expect(listDiff.additions[0].vin).to.equal('3');
      expect(listDiff.deletions).to.be.an('array');
      expect(listDiff.deletions[0]).to.equal('2');
    });
  });
});


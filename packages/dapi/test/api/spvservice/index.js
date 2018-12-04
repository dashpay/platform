process.on('unhandledRejection', (up) => {
  throw up;
});
const BloomFilter = require('bloom-filter');
const chai = require('chai');
const sinon = require('sinon');
const index = require('../../../lib/api/spvservice/index');

const { expect } = chai;

let spy;

describe('spvservice/index', () => {
  describe('#factory', () => {
    const filter = BloomFilter.create(3, 0.01);

    it('should be loadBloomFilter function', () => {
      const res = index.loadBloomFilter;
      expect(res).to.be.a('function');
    });
    it('should be addToBloomFilter function', () => {
      const res = index.addToBloomFilter;
      expect(res).to.be.a('function');
    });

    it('should be clearBloomFilter function', () => {
      const res = index.clearBloomFilter;
      expect(res).to.be.a('function');
    });

    it('should be getSpvData function', () => {
      const res = index.getSpvData;
      expect(res).to.be.a('function');
    });

    it('should be findDataForBlock function', () => {
      const res = index.getSpvData;
      expect(res).to.be.a('function');
    });

    it('should call loadBloomFilter with valid BloomFilter', () => {
      spy = sinon.spy(index, 'loadBloomFilter');
      index.loadBloomFilter(filter);
      expect(spy.callCount).to.be.equal(1);
    });

    it('should call loadBloomFilter with valid BloomFilter', () => {
      spy = sinon.spy(index, 'addToBloomFilter');
      index.addToBloomFilter(filter);
      expect(spy.callCount).to.be.equal(1);
    });

    it('should call clearBloomFilter with valid BloomFilter', () => {
      spy = sinon.spy(index, 'clearBloomFilter');
      index.clearBloomFilter(filter);
      expect(spy.callCount).to.be.equal(1);
    });

    it('should call getSpvData with valid BloomFilter', () => {
      spy = sinon.spy(index, 'getSpvData');
      index.getSpvData(filter);
      expect(spy.callCount).to.be.equal(1);
    });

    it('should call findDataForBlock with valid BloomFilter', () => {
      spy = sinon.spy(index, 'findDataForBlock');
      index.findDataForBlock(filter);
      expect(spy.callCount).to.be.equal(1);
    });
  });
});

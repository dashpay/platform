const { expect } = require('chai');
const InMem = require('./InMem');

const inMem = new InMem();

describe('Adapter - inMem', function suite() {
  this.timeout(10000);
  it('should provide a config method', () => {
    expect(inMem.config).to.exist;
  });
  it('should set an item', () => {
    const item = { item: 'item' };
    expect(inMem.setItem('toto', item)).to.deep.equal(item);
  });
  it('should get an item', () => {
    const item = { item: 'item' };
    expect(inMem.getItem('toto')).to.deep.equal(item);
  });
});

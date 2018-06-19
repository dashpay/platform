const DapObject = require('../../../../lib/stateView/dapObject/DapObject');

describe('DapObject', () => {
  it('should serialize DapObject', () => {
    const id = '123456';
    const dapObject = new DapObject(id);

    const dapObjectSerialized = dapObject.toJSON();
    expect(dapObjectSerialized).to.deep.equal({
      id,
    });
  });
});

const Reference = require('../../../../lib/stateView/Reference');
const DapObject = require('../../../../lib/stateView/dapObject/DapObject');

describe('DapObject', () => {
  it('should serialize DapObject', () => {
    const blockHash = 'b8ae412cdeeb4bb39ec496dec34495ecccaf74f9fa9eaa712c77a03eb1994e75';
    const blockHeight = 1;
    const headerHash = '17jasdjk129uasd8asd023098SD09023jll123jlasd90823jklD';
    const hashSTPacket = 'ad877138as8012309asdkl123l123lka908013';
    const reference = new Reference(
      blockHash,
      blockHeight,
      headerHash,
      hashSTPacket,
    );
    const dapObjectData = {
      id: '1234',
      objtype: 'user',
      idx: 0,
      rev: 1,
      act: 1,
    };
    const dapObject = new DapObject(dapObjectData, reference);

    const dapObjectSerialized = dapObject.toJSON();
    expect(dapObjectSerialized).to.deep.equal({
      id: dapObjectData.id,
      type: dapObjectData.objtype,
      object: dapObjectData,
      revision: dapObjectData.rev,
      reference: reference.toJSON(),
    });
  });

  it('should be new if DapObject action is 0', async () => {
    const reference = new Reference();
    const dapObjectData = {
      id: '1234',
      objtype: 'user',
      idx: 0,
      rev: 1,
      act: 0,
    };
    const dapObject = new DapObject(dapObjectData, reference);
    expect(dapObject.isNew()).to.be.true();
  });

  it('should not be new if DapObject action is not 0', async () => {
    const reference = new Reference();
    const dapObjectData = {
      id: '1234',
      objtype: 'user',
      idx: 0,
      rev: 1,
      act: 1,
    };
    const dapObject = new DapObject(dapObjectData, reference);
    expect(dapObject.isNew()).to.be.false();
  });
});

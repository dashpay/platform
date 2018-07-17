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

  it('should have DapObject action constants', async () => {
    expect(DapObject.ACTION_CREATE).to.be.equal(0);
    expect(DapObject.ACTION_UPDATE).to.be.equal(1);
    expect(DapObject.ACTION_DELETE).to.be.equal(2);
  });

  it('should get DapObject action', async () => {
    const reference = new Reference();
    const dapObjectData = {
      id: '1234',
      objtype: 'user',
      idx: 0,
      rev: 1,
      act: 0,
    };
    const dapObject = new DapObject(dapObjectData, reference);
    expect(dapObject.getAction()).to.be.equal(0);
  });
});

const Reference = require('../../../../lib/stateView/Reference');
const updateDapObjectFactory = require('../../../../lib/stateView/dapObject/updateDapObjectFactory');

describe('updateDapObjectFactory', () => {
  let dapObjectRepository;
  let updateDapObject;
  let reference;

  beforeEach(function beforeEach() {
    dapObjectRepository = {
      store: this.sinon.stub(),
    };
    const createDapObjectRepository = () => dapObjectRepository;
    updateDapObject = updateDapObjectFactory(createDapObjectRepository);

    const blockHash = 'b8ae412cdeeb4bb39ec496dec34495ecccaf74f9fa9eaa712c77a03eb1994e75';
    const blockHeight = 1;
    const headerHash = '17jasdjk129uasd8asd023098SD09023jll123jlasd90823jklD';
    const hashSTPacket = 'ad877138as8012309asdkl123l123lka908013';
    reference = new Reference(
      blockHash,
      blockHeight,
      headerHash,
      hashSTPacket,
    );
  });

  it('should store DapObject if action is 0', async () => {
    const dapId = '1234';
    const dapObject = {
      objtype: 'user',
      idx: 0,
      rev: 1,
      act: 0,
    };
    updateDapObject(dapId, reference, dapObject);
    expect(dapObjectRepository.store).to.calledOnce();
  });

  it('should store DapObject if action is 1', async () => {
    const dapId = '1234';
    const dapObject = {
      objtype: 'user',
      idx: 0,
      rev: 1,
      act: 1,
    };
    updateDapObject(dapId, reference, dapObject);
    expect(dapObjectRepository.store).to.calledOnce();
  });

  it('should not store DapContract if action is not 0 or 1', async () => {
    const dapId = '1234';
    const dapObject = {
      objtype: 'user',
      idx: 0,
      rev: 1,
      act: 5,
    };
    updateDapObject(dapId, reference, dapObject);
    expect(dapObjectRepository.store).to.not.calledOnce();
  });
});

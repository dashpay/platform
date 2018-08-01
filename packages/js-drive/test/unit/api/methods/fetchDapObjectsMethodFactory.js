const fetchDapObjectsMethodFactory = require('../../../../lib/api/methods/fetchDapObjectsMethodFactory');
const InvalidParamsError = require('../../../../lib/api/InvalidParamsError');
const DapObject = require('../../../../lib/stateView/dapObject/DapObject');
const Reference = require('../../../../lib/stateView/Reference');
const InvalidWhereError = require('../../../../lib/stateView/dapObject/InvalidWhereError');
const InvalidOrderByError = require('../../../../lib/stateView/dapObject/InvalidOrderByError');
const InvalidLimitError = require('../../../../lib/stateView/dapObject/InvalidLimitError');
const InvalidStartAtError = require('../../../../lib/stateView/dapObject/InvalidStartAtError');
const InvalidStartAfterError = require('../../../../lib/stateView/dapObject/InvalidStartAfterError');

describe('fetchDapObjectsMethod', () => {
  let fetchDapObjects;
  let fetchDapObjectsMethod;

  beforeEach(function beforeEach() {
    fetchDapObjects = this.sinon.stub();
    fetchDapObjectsMethod = fetchDapObjectsMethodFactory(fetchDapObjects);
  });

  it('should throw InvalidParamsError if DAP id is not provided', () => {
    expect(fetchDapObjectsMethod()).to.be.rejectedWith(InvalidParamsError);
  });

  it('should throw InvalidParamsError if InvalidWhereError is thrown', () => {
    const dapId = 'b8ae412cdeeb4bb39ec496dec34495ecccaf74f9fa9eaa712c77a03eb1994e75';
    const type = 'DashPayContact';
    const options = {};
    fetchDapObjects.throws(new InvalidWhereError());
    expect(fetchDapObjectsMethod({ dapId, type, options })).to.be.rejectedWith(InvalidParamsError);
  });

  it('should throw InvalidParamsError if InvalidOrderByError is thrown', () => {
    const dapId = 'b8ae412cdeeb4bb39ec496dec34495ecccaf74f9fa9eaa712c77a03eb1994e75';
    const type = 'DashPayContact';
    const options = {};
    fetchDapObjects.throws(new InvalidOrderByError());
    expect(fetchDapObjectsMethod({ dapId, type, options })).to.be.rejectedWith(InvalidParamsError);
  });

  it('should throw InvalidParamsError if InvalidLimitError is thrown', () => {
    const dapId = 'b8ae412cdeeb4bb39ec496dec34495ecccaf74f9fa9eaa712c77a03eb1994e75';
    const type = 'DashPayContact';
    const options = {};
    fetchDapObjects.throws(new InvalidLimitError());
    expect(fetchDapObjectsMethod({ dapId, type, options })).to.be.rejectedWith(InvalidParamsError);
  });

  it('should throw InvalidParamsError if InvalidStartAtError is thrown', () => {
    const dapId = 'b8ae412cdeeb4bb39ec496dec34495ecccaf74f9fa9eaa712c77a03eb1994e75';
    const type = 'DashPayContact';
    const options = {};
    fetchDapObjects.throws(new InvalidStartAtError());
    expect(fetchDapObjectsMethod({ dapId, type, options })).to.be.rejectedWith(InvalidParamsError);
  });

  it('should throw InvalidParamsError if InvalidStartAfterError is thrown', () => {
    const dapId = 'b8ae412cdeeb4bb39ec496dec34495ecccaf74f9fa9eaa712c77a03eb1994e75';
    const type = 'DashPayContact';
    const options = {};
    fetchDapObjects.throws(new InvalidStartAfterError());
    expect(fetchDapObjectsMethod({ dapId, type, options })).to.be.rejectedWith(InvalidParamsError);
  });

  it('should return DAP object', async () => {
    const id = '123456';
    const objectData = {
      id,
      act: 0,
      objtype: 'DashPayContact',
      user: 'dashy',
      rev: 0,
    };
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
    const dapObject = new DapObject(objectData, reference);
    fetchDapObjects.returns([dapObject]);

    const dapId = 'b8ae412cdeeb4bb39ec496dec34495ecccaf74f9fa9eaa712c77a03eb1994e75';
    const type = 'DashPayContact';
    const dapObjects = await fetchDapObjectsMethod({ dapId, type });

    expect(dapObjects[0]).to.be.deep.equal(dapObject.toJSON());
  });
});

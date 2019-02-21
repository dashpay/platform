const fetchDPObjectsMethodFactory = require('../../../../lib/api/methods/fetchDPObjectsMethodFactory');

const getDPObjectsFixture = require('../../../../lib/test/fixtures/getDPObjectsFixture');

const InvalidParamsError = require('../../../../lib/api/InvalidParamsError');
const InvalidWhereError = require('../../../../lib/stateView/object/errors/InvalidWhereError');
const InvalidOrderByError = require('../../../../lib/stateView/object/errors/InvalidOrderByError');
const InvalidLimitError = require('../../../../lib/stateView/object/errors/InvalidLimitError');
const InvalidStartAtError = require('../../../../lib/stateView/object/errors/InvalidStartAtError');
const InvalidStartAfterError = require('../../../../lib/stateView/object/errors/InvalidStartAfterError');
const AmbiguousStartError = require('../../../../lib/stateView/object/errors/AmbiguousStartError');

describe('fetchDPObjectsMethodFactory', () => {
  let contractId;
  let type;
  let options;
  let fetchDPObjectsMock;
  let fetchDPObjectsMethod;

  async function throwErrorAndExpectInvalidParamError(error) {
    fetchDPObjectsMock.throws(error);

    let actualError;
    try {
      await fetchDPObjectsMethod({ contractId, type, options });
    } catch (e) {
      actualError = e;
    }

    expect(actualError).to.be.an.instanceOf(InvalidParamsError);

    expect(fetchDPObjectsMock).to.have.been.calledOnceWith(contractId, type, options);
  }

  beforeEach(function beforeEach() {
    contractId = 'b8ae412cdeeb4bb39ec496dec34495ecccaf74f9fa9eaa712c77a03eb1994e75';
    type = 'niceObject';
    options = {};

    fetchDPObjectsMock = this.sinon.stub();
    fetchDPObjectsMethod = fetchDPObjectsMethodFactory(fetchDPObjectsMock);
  });

  it('should throw InvalidParamsError if Contract ID is not provided', async () => {
    let error;
    try {
      await fetchDPObjectsMethod({});
    } catch (e) {
      error = e;
    }

    expect(error).to.be.an.instanceOf(InvalidParamsError);

    expect(fetchDPObjectsMock).to.have.not.been.called();
  });

  it('should throw InvalidParamsError if InvalidWhereError is thrown', async () => {
    await throwErrorAndExpectInvalidParamError(new InvalidWhereError());
  });

  it('should throw InvalidParamsError if InvalidOrderByError is thrown', async () => {
    await throwErrorAndExpectInvalidParamError(new InvalidOrderByError());
  });

  it('should throw InvalidParamsError if InvalidLimitError is thrown', async () => {
    await throwErrorAndExpectInvalidParamError(new InvalidLimitError());
  });

  it('should throw InvalidParamsError if InvalidStartAtError is thrown', async () => {
    await throwErrorAndExpectInvalidParamError(new InvalidStartAtError());
  });

  it('should throw InvalidParamsError if InvalidStartAfterError is thrown', async () => {
    await throwErrorAndExpectInvalidParamError(new InvalidStartAfterError());
  });

  it('should throw InvalidParamsError if AmbiguousStartError is thrown', async () => {
    await throwErrorAndExpectInvalidParamError(new AmbiguousStartError());
  });

  it('should escalate an error if error type is unknown', async () => {
    const fetchError = new Error();

    fetchDPObjectsMock.throws(fetchError);

    let error;
    try {
      await fetchDPObjectsMethod({ contractId, type, options });
    } catch (e) {
      error = e;
    }

    expect(error).to.equal(fetchError);

    expect(fetchDPObjectsMock).to.have.been.calledOnceWith(contractId, type, options);
  });

  it('should return DP Objects', async () => {
    const dpObjects = getDPObjectsFixture();
    const rawDPObjects = dpObjects.map(o => o.toJSON());

    fetchDPObjectsMock.resolves(dpObjects);

    const result = await fetchDPObjectsMethod({ contractId, type, options });

    expect(result).to.deep.equal(rawDPObjects);

    expect(fetchDPObjectsMock).to.have.been.calledOnceWith(contractId, type, options);
  });
});

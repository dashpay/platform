const InvalidSTPacketError = require('@dashevo/dpp/lib/stPacket/errors/InvalidSTPacketError');

const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');

const StateTransition = require('../../../../lib/blockchain/StateTransition');

const addSTPacketMethodFactory = require('../../../../lib/api/methods/addSTPacketMethodFactory');

const getSTPacketsFixture = require('../../../../lib/test/fixtures/getSTPacketsFixture');
const getStateTransitionsFixture = require('../../../../lib/test/fixtures/getStateTransitionsFixture');

const InvalidParamsError = require('../../../../lib/api/InvalidParamsError');

describe('addSTPacketMethodFactory', () => {
  let stPacket;
  let serializedSTPacket;
  let stateTransition;
  let serializedStateTransition;
  let dppMock;
  let addSTPacketMock;
  let addSTPacketMethod;

  beforeEach(function beforeEach() {
    [stPacket] = getSTPacketsFixture();
    [stateTransition] = getStateTransitionsFixture();

    serializedSTPacket = stPacket.serialize().toString('hex');
    serializedStateTransition = stateTransition.serialize();

    dppMock = createDPPMock(this.sinon);

    addSTPacketMock = this.sinon.stub();

    addSTPacketMethod = addSTPacketMethodFactory(addSTPacketMock, dppMock);
  });

  it('should throw error if "stPacket" params is missing', async () => {
    let error;
    try {
      await addSTPacketMethod({
        stateTransition: serializedStateTransition,
      });
    } catch (e) {
      error = e;
    }

    expect(error).to.be.instanceOf(InvalidParamsError);

    expect(addSTPacketMock).to.not.be.called();
  });

  it('should throw error if "stateTransition" params is missing', async () => {
    let error;
    try {
      await addSTPacketMethod({
        stPacket: serializedSTPacket,
      });
    } catch (e) {
      error = e;
    }

    expect(error).to.be.instanceOf(InvalidParamsError);

    expect(addSTPacketMock).to.not.be.called();
  });

  it('should throw error if "stPacket" param is not a serialized ST Packet', async () => {
    const wrongString = 'something';

    const cborError = new Error();

    dppMock.packet.createFromSerialized.throws(cborError);

    let error;
    try {
      await addSTPacketMethod({
        stateTransition: serializedStateTransition,
        stPacket: wrongString,
      });
    } catch (e) {
      error = e;
    }

    expect(error).to.be.equal(cborError);

    expect(dppMock.packet.createFromSerialized).to.be.calledOnceWith(wrongString);

    expect(addSTPacketMock).to.not.be.called();
  });

  it('should throw error if "stateTransition" param is not a serialized ST', async () => {
    const wrongString = 'something';

    const cborError = new Error();

    dppMock.packet.createFromSerialized.throws(cborError);

    let error;
    try {
      await addSTPacketMethod({
        stateTransition: wrongString,
        stPacket: serializedSTPacket,
      });
    } catch (e) {
      error = e;
    }

    expect(error).to.be.equal(cborError);

    expect(dppMock.packet.createFromSerialized).to.be.calledOnceWith(serializedSTPacket);

    expect(addSTPacketMock).to.not.be.called();
  });

  it('should throw error if "stPacket" param is not valid ST Packet', async () => {
    const invalidSTPacket = { ...stPacket.toJSON(), wrongField: true };

    const validationError = new InvalidSTPacketError([], invalidSTPacket);

    dppMock.packet.createFromSerialized.throws(validationError);

    let error;
    try {
      await addSTPacketMethod({
        stateTransition: serializedStateTransition,
        stPacket: serializedSTPacket,
      });
    } catch (e) {
      error = e;
    }

    expect(error).to.be.instanceOf(InvalidParamsError);

    expect(dppMock.packet.createFromSerialized).to.be.calledOnceWith(serializedSTPacket);

    expect(addSTPacketMock).to.not.be.called();
  });

  it('should add ST Packet', async () => {
    dppMock.packet.createFromSerialized.resolves(stPacket);

    await addSTPacketMethod({
      stateTransition: serializedStateTransition,
      stPacket: serializedSTPacket,
    });

    expect(dppMock.packet.createFromSerialized).to.be.calledOnceWith(serializedSTPacket);

    expect(addSTPacketMock).to.be.calledOnceWith(stPacket);
    expect(addSTPacketMock.getCall(0).args).to.have.lengthOf(2);

    const passedStateTransition = addSTPacketMock.getCall(0).args[1];

    expect(passedStateTransition).to.be.instanceOf(StateTransition);
    expect(passedStateTransition.hash).to.be.equal(stateTransition.hash);
  });
});

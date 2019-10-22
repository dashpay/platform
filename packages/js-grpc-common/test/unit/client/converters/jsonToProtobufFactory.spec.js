
const jsonToProtobufFactory = require(
  '../../../../lib/client/converters/jsonToProtobufFactory',
);

describe('jsonToProtobufFactory', () => {
  let protocClassMock;
  let pbjsClassMock;
  let finishMock;
  let jsonToProtobuf;

  beforeEach(function beforeEach() {
    protocClassMock = {
      deserializeBinary: this.sinon.stub(),
    };

    finishMock = this.sinon.stub();

    pbjsClassMock = {
      fromObject: this.sinon.stub(),
      encode: this.sinon.spy(() => ({
        finish: finishMock,
      })),
    };

    jsonToProtobuf = jsonToProtobufFactory(protocClassMock, pbjsClassMock);
  });

  it('should call methods of the provided classes', () => {
    const message = {
      value: 'message',
    };

    const grpcMessage = {
      value: 'grpcMessage',
    };

    const grpcMessageBinary = {
      value: 'grpcMessageBinary',
    };

    const converted = {
      value: 'result',
    };

    protocClassMock.deserializeBinary.returns(converted);
    pbjsClassMock.fromObject.returns(grpcMessage);
    finishMock.returns(grpcMessageBinary);

    const result = jsonToProtobuf(message);

    expect(result).to.deep.equal(converted);

    expect(pbjsClassMock.fromObject).to.have.been.calledOnceWith(message);
    expect(pbjsClassMock.encode).to.have.been.calledOnceWith(grpcMessage);
    expect(finishMock).to.have.been.calledOnce();

    expect(protocClassMock.deserializeBinary).to.have.been.calledOnceWith(grpcMessageBinary);
  });
});

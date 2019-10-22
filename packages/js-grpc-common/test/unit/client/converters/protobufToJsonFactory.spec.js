const protobufToJsonFactory = require(
  '../../../../lib/client/converters/protobufToJsonFactory',
);

describe('protobufToJsonFactory', () => {
  let pbjsClassMock;
  let protobufToJson;

  beforeEach(function beforeEach() {
    pbjsClassMock = {
      decode: this.sinon.stub(),
      toObject: this.sinon.stub(),
    };

    protobufToJson = protobufToJsonFactory(pbjsClassMock);
  });

  it('should call PBJS class methods', function it() {
    const serializedBinary = {
      value: 'serializedBinary',
    };

    const message = {
      value: 'message',
      serializeBinary: this.sinon.spy(() => serializedBinary),
    };

    const grpcMessage = {
      value: 'grpcMessage',
    };

    const converted = {
      value: 'converted',
    };

    pbjsClassMock.decode.returns(grpcMessage);
    pbjsClassMock.toObject.returns(converted);

    const result = protobufToJson(message);

    expect(result).to.deep.equal(converted);

    expect(message.serializeBinary).to.have.been.called();
    expect(pbjsClassMock.decode).to.have.been.calledOnceWith(serializedBinary);
    expect(pbjsClassMock.toObject).to.have.been.calledOnceWith(grpcMessage);
  });
});

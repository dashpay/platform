/**
 * Convert snake cased json object to protobuf message (factory)
 *
 * @param ProtocMessageClass
 * @param PBJSMessageClass
 *
 * @returns {jsonToProtobuf}
 */
function jsonToProtobufFactory(ProtocMessageClass, PBJSMessageClass) {
  /**
   * Convert snake cased json object to protobuf message
   *
   * @typedef jsonToProtobuf
   *
   * @param {Object} object
   *
   * @returns {*}
   */
  function jsonToProtobuf(object) {
    const grpcMessage = PBJSMessageClass.fromObject(object);
    const grpcMessageBinary = PBJSMessageClass
      .encode(grpcMessage)
      .finish();

    return ProtocMessageClass.deserializeBinary(grpcMessageBinary);
  }

  return jsonToProtobuf;
}

module.exports = jsonToProtobufFactory;

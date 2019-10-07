/**
 * Converts protobuf message to a JSON (factory)
 *
 * @param PBJSMessageClass
 *
 * @returns {Object}
 */
function protobufToJsonFactory(PBJSMessageClass) {
  /**
   * Converts protobuf message to a JSON
   *
   * @typedef protobufToJson
   *
   * @param message
   *
   * @returns {Object}
   */
  function protobufToJson(message) {
    const messageBinary = message.serializeBinary();
    const grpcMessage = PBJSMessageClass
      .decode(messageBinary);

    return PBJSMessageClass.toObject(grpcMessage);
  }

  return protobufToJson;
}

module.exports = protobufToJsonFactory;

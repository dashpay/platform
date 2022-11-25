const timeToMillis = require('./timeToMillis');

/**
 * @param {google.protobuf.ITimestamp} timestamp
 * @returns {number}
 */
function protoTimestampToMillis(timestamp) {
  return timeToMillis(timestamp.seconds.toNumber(), timestamp.nanos);
}

module.exports = protoTimestampToMillis;

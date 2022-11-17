const {
  google: {
    protobuf: {
      Timestamp,
    },
  },
} = require('@dashevo/abci/types');

const Long = require('long');

/**
 * Get milliseconds time from seconds and nanoseconds
 *
 * @param {number} milliseconds
 *
 * @returns {number}
 */
function millisToProtoTimestamp(milliseconds) {
  const seconds = Math.floor(milliseconds / 1000);
  const nanos = (milliseconds - (seconds * 1000)) * (10 ** 6);

  return new Timestamp({
    seconds: Long.fromNumber(seconds),
    nanos,
  });
}

module.exports = millisToProtoTimestamp;

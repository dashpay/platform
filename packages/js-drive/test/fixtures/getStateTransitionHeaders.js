const fs = require('fs');

module.exports = function loadStateTransitionPackets() {
  const packetsJSON = fs.readFileSync(`${__dirname}/stateTransitionHeaders.json`);
  return JSON.parse(packetsJSON);
};

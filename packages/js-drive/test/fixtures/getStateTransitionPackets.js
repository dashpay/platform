const fs = require('fs');

module.exports = function loadStateTransitionPackets() {
  const packetsJSON = fs.readFileSync(`${__dirname}/stateTransitionPackets.json`);
  return JSON.parse(packetsJSON);
};

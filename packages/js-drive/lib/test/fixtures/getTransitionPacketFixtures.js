const fs = require('fs');

const StateTransitionPacket = require('../../storage/StateTransitionPacket');

module.exports = function getTransitionPacketFixtures() {
  const packetsJSON = fs.readFileSync(`${__dirname}/../../../test/fixtures/stateTransitionPackets.json`);
  const transitionPacketPacketData = JSON.parse(packetsJSON);

  return transitionPacketPacketData.map(packet => new StateTransitionPacket(packet));
};

const fs = require('fs');

const StateTransitionPacket = require('../../storage/stPacket/StateTransitionPacket');

module.exports = function getTransitionPacketFixtures() {
  const packetsJSON = fs.readFileSync(`${__dirname}/../../../test/fixtures/stateTransitionPackets.json`);
  const transitionPacketPacketData = JSON.parse(packetsJSON.toString());

  return transitionPacketPacketData.map(packet => new StateTransitionPacket(packet));
};

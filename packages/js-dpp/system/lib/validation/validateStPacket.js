const Ajv = require('ajv');

const validatePacket = require('./stPacket/validatePacket');
const validatePacketObjects = require('./stPacket/validatePacketObjects');

const dashSchema = require('../../../dash-schema/schema/schema');
const stPacketSchema = require('../../schema/st-packet');
const dapObjectSchema = require('../../schema/dap-object');
const appContractMetaSchema = require('../../schema/dap-contract');

module.exports = function validateStPacket(packet, appContract) {
  const ajv = new Ajv();

  ajv.addSchema(dashSchema);
  ajv.addSchema(stPacketSchema);
  ajv.addSchema(dapObjectSchema);
  ajv.addSchema(appContractMetaSchema);

  ajv.addSchema(appContract, 'dap-contract');

  let errors = validatePacket(ajv, packet);

  if (errors) {
    return errors;
  }

  if (packet.contracts)

  if (packet.objects) {
    errors = validatePacketObjects(ajv, packet.objects);
  }



  if (errors.length) {
    return errors;
  }
};

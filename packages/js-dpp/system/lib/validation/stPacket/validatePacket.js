function validatePacket(ajv, packet) {
  ajv.validate(
    'https://schema.dash.org/platform-4-0-0/system/st-packet',
    packet,
  );

  return ajv.errors;
}

module.exports = validatePacket;

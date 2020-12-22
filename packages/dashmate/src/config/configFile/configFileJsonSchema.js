module.exports = {
  $schema: 'http://json-schema.org/draft-07/schema#',
  type: 'object',
  properties: {
    configFormatVersion: {
      type: 'string',
    },
    defaultConfigName: {
      type: ['string', 'null'],
    },
    configs: {
      type: 'object',
    },
  },
  required: ['configFormatVersion', 'defaultConfigName', 'configs'],
  additionalProperties: false,
};
